// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{collections::BTreeSet, str::FromStr};

use anyhow::{anyhow, Result};
use sui_keys::keystore::AccountKeystore;
use sui_sdk::{
    rpc_types::{
        DryRunTransactionBlockResponse,
        SuiTransactionBlockEffectsAPI,
        SuiTransactionBlockResponse,
    },
    wallet_context::WalletContext,
    SuiClient,
};
use sui_types::{
    base_types::{ObjectID, ObjectRef, SuiAddress},
    transaction::{CallArg, ProgrammableTransaction},
    Identifier,
};

use super::{
    builder::SitePtb,
    resource::ResourceOp,
    RemoteSiteFactory,
    SiteData,
    SiteDataDiff,
    SITE_MODULE,
};
use crate::{
    display,
    publish::WhenWalrusUpload,
    summary::SiteDataDiffSummary,
    util::{get_site_id_from_response, sign_and_send_ptb},
    walrus::Walrus,
    Config,
};

const MAX_RESOURCES_PER_PTB: usize = 200;

/// The identifier for the new or existing site.
///
/// Either object ID (existing site) or name (new site).
#[derive(Debug, Clone)]
pub enum SiteIdentifier {
    ExistingSite(ObjectID),
    NewSite(String),
}

pub struct SiteManager {
    pub config: Config,
    pub walrus: Walrus,
    pub wallet: WalletContext,
    pub site_id: SiteIdentifier,
    pub epochs: u64,
    pub when_upload: WhenWalrusUpload,
}

impl SiteManager {
    /// Creates a new site manager.
    pub async fn new(
        config: Config,
        walrus: Walrus,
        wallet: WalletContext,
        site_id: SiteIdentifier,
        epochs: u64,
        when_upload: WhenWalrusUpload,
    ) -> Result<Self> {
        Ok(SiteManager {
            walrus,
            wallet,
            config,
            site_id,
            epochs,
            when_upload,
        })
    }

    /// Updates the site with the given [`Resource`](super::resource::Resource).
    ///
    /// If the site does not exist, it is created and updated. The resources that need to be updated
    /// or created are published to Walrus.
    pub async fn update_site(
        &mut self,
        local_site_data: &SiteData,
    ) -> Result<(SuiTransactionBlockResponse, SiteDataDiffSummary)> {
        tracing::debug!(?self.site_id, "creating or updating site");
        let existing_site = match &self.site_id {
            SiteIdentifier::ExistingSite(site_id) => {
                RemoteSiteFactory::new(&self.sui_client().await?, self.config.package)
                    .await?
                    .get_from_chain(*site_id)
                    .await?
            }
            SiteIdentifier::NewSite(_) => SiteData::empty(),
        };
        tracing::debug!(?existing_site, "checked existing site");

        let site_updates = if self.when_upload.is_always() {
            existing_site.replace_all(local_site_data)
        } else {
            local_site_data.diff(&existing_site)
        };
        tracing::debug!(operations=?site_updates, "list of operations computed");

        let walrus_updates = site_updates.get_walrus_updates(&self.when_upload);
        let mut total_storage_cost = 0;

        if !walrus_updates.is_empty() {
            tracing::info!("Dry-running Walrus store operations");
            for update in &walrus_updates {
                let resource = update.inner();

                let dry_run_output = self
                    .walrus
                    .dry_run_store(resource.full_path.clone(), self.epochs, false)
                    .await?;

                let storage_cost = dry_run_output.storage_cost;
                total_storage_cost += storage_cost;
            }
        }

        // Check if there are any updates to the site on-chain.
        let result = if site_updates.has_updates() {
            // Before doing the actual execution, perform a dry run
            let gas_summary = self.dry_run_sui_updates(&site_updates).await?;
            tracing::info!(
                "Estimated costs for this publish/update: Sui Gas Cost {} MIST, Storage Cost: {} FROST",
                gas_summary,
                total_storage_cost
            );

            // Add user confirmation prompt.
            display::action("Waiting for user confirmation...");
            if !dialoguer::Confirm::new()
                .with_prompt("Do you want to proceed with these updates?")
                .default(true)
                .interact()?
            {
                display::error("Update cancelled by user");
                return Err(anyhow!("Update cancelled by user"));
            }
            display::action("Applying the Walrus Site object updates on Sui");
            let result = self.execute_sui_updates(&site_updates).await?;
            display::done();
            result
        } else {
            // No updates necessary
            SuiTransactionBlockResponse::default()
        };

        // After applying on-chain updates, publish to Walrus if needed.
        if !walrus_updates.is_empty() {
            self.publish_to_walrus(&walrus_updates).await?;
        }

        Ok((result, site_updates.summary(&self.when_upload)))
    }

    /// Publishes the resources to Walrus.
    async fn publish_to_walrus<'b>(&mut self, updates: &[&ResourceOp<'b>]) -> Result<()> {
        for update in updates.iter() {
            let resource = update.inner();
            tracing::debug!(
                resource=?resource.full_path,
                blob_id=%resource.info.blob_id,
                unencoded_size=%resource.unencoded_size,
                "storing new blob on Walrus"
            );
            display::action(format!(
                "Storing resource on Walrus: {}",
                &resource.info.path
            ));
            let _output = self
                .walrus
                .store(resource.full_path.clone(), self.epochs, false)
                .await?;
            display::done();
        }
        Ok(())
    }

    /// Executes the updates on Sui.
    async fn execute_sui_updates<'b>(
        &self,
        updates: &SiteDataDiff<'b>,
    ) -> Result<SuiTransactionBlockResponse> {
        tracing::debug!(
            address=?self.active_address()?,
            "starting to update site resources on chain",
        );
        let ptb = SitePtb::new(
            self.config.package,
            Identifier::from_str(SITE_MODULE).expect("the str provided is valid"),
        )?;

        // Add the call arg if we are updating a site, or add the command to create a new site.
        let mut ptb = match &self.site_id {
            SiteIdentifier::ExistingSite(site_id) => {
                ptb.with_call_arg(&self.wallet.get_object_ref(*site_id).await?.into())?
            }
            SiteIdentifier::NewSite(site_name) => ptb.with_create_site(site_name)?,
        };

        // Publish the first MAX_RESOURCES_PER_PTB resources, or all resources if there are fewer
        // than that.
        tracing::debug!("preparing and committing the first PTB");
        let mut end = MAX_RESOURCES_PER_PTB.min(updates.resource_ops.len());

        ptb.add_resource_operations(&updates.resource_ops[..end])?;
        ptb.add_route_operations(&updates.route_ops)?;

        if self.needs_transfer() {
            ptb.transfer_site(self.active_address()?);
        }

        let result = self
            .sign_and_send_ptb(ptb.finish(), self.gas_coin_ref().await?)
            .await?;

        let site_object_id = match &self.site_id {
            SiteIdentifier::ExistingSite(site_id) => *site_id,
            SiteIdentifier::NewSite(_) => {
                let resp = result
                    .effects
                    .as_ref()
                    .ok_or(anyhow!("the result did not have effects"))?;
                get_site_id_from_response(self.active_address()?, resp)?
            }
        };

        // Keep iterating to load resources
        while end < updates.resource_ops.len() {
            let start = end;
            end = (end + MAX_RESOURCES_PER_PTB).min(updates.resource_ops.len());
            tracing::debug!(%start, %end, "preparing and committing the next PTB");

            let ptb = SitePtb::new(
                self.config.package,
                Identifier::from_str(SITE_MODULE).expect("the str provided is valid"),
            )?;
            let call_arg: CallArg = self.wallet.get_object_ref(site_object_id).await?.into();
            let mut ptb = ptb.with_call_arg(&call_arg)?;
            ptb.add_resource_operations(&updates.resource_ops[start..end])?;

            let _result = self
                .sign_and_send_ptb(ptb.finish(), self.gas_coin_ref().await?)
                .await?;
        }

        Ok(result)
    }

    async fn sign_and_send_ptb(
        &self,
        programmable_transaction: ProgrammableTransaction,
        gas_coin: ObjectRef,
    ) -> Result<SuiTransactionBlockResponse> {
        sign_and_send_ptb(
            self.active_address()?,
            &self.wallet,
            programmable_transaction,
            gas_coin,
            self.config.gas_budget(),
        )
        .await
    }

    /// Dry runs the updates on Sui without committing them.
    /// Returns the `DryRunTransactionBlockResponse` so that you can inspect.
    /// the effects, costs, and outcome of the transaction sequence.
    async fn dry_run_sui_updates<'b>(&self, updates: &SiteDataDiff<'b>) -> Result<u64> {
        tracing::debug!(
            address = ?self.active_address()?,
            "starting to dry-run update site resources on chain",
        );

        let ptb = SitePtb::new(
            self.config.package,
            Identifier::from_str(SITE_MODULE).expect("the str provided is valid"),
        )?;

        // Add the call arg if we are updating a site, or add the command to create a new site.
        let mut ptb = match &self.site_id {
            SiteIdentifier::ExistingSite(site_id) => {
                ptb.with_call_arg(&self.wallet.get_object_ref(*site_id).await?.into())?
            }
            SiteIdentifier::NewSite(site_name) => ptb.with_create_site(site_name)?,
        };

        // Add as many resources as fit into the first PTB.
        tracing::debug!("preparing and committing the first PTB");
        let mut end = MAX_RESOURCES_PER_PTB.min(updates.resource_ops.len());

        ptb.add_resource_operations(&updates.resource_ops[..end])?;
        ptb.add_route_operations(&updates.route_ops)?;

        if self.needs_transfer() {
            ptb.transfer_site(self.active_address()?);
        }

        let first_ptb = ptb.finish();
        let gas_coin = self.gas_coin_ref().await?;
        let dry_run_response = self.dry_run_ptb(first_ptb, gas_coin).await?;
        let site_object_id = match &self.site_id {
            SiteIdentifier::ExistingSite(site_id) => *site_id,
            SiteIdentifier::NewSite(_) => {
                get_site_id_from_response(self.active_address()?, &dry_run_response.effects)?
            }
        };
        // Track total storage costs.
        let mut total_storage_cost = dry_run_response.effects.gas_cost_summary().storage_cost;
        total_storage_cost += dry_run_response.effects.gas_cost_summary().computation_cost;

        // Continue processing additional resource operations in batches, if any.
        while end < updates.resource_ops.len() {
            let start = end;
            end = (end + MAX_RESOURCES_PER_PTB).min(updates.resource_ops.len());
            tracing::debug!(%start, %end, "preparing and committing the next PTB");

            let ptb = SitePtb::new(
                self.config.package,
                Identifier::from_str(SITE_MODULE).expect("the str provided is valid"),
            )?;
            let call_arg: CallArg = self.wallet.get_object_ref(site_object_id).await?.into();
            let mut ptb = ptb.with_call_arg(&call_arg)?;
            ptb.add_resource_operations(&updates.resource_ops[start..end])?;

            // Dry run the next batch and add its storage cost to total.
            let next_response = self
                .dry_run_ptb(ptb.finish(), self.gas_coin_ref().await?)
                .await?;
            total_storage_cost += next_response.effects.gas_cost_summary().storage_cost;
            total_storage_cost += next_response.effects.gas_cost_summary().computation_cost;
        }

        tracing::debug!("Total storage cost: {}", total_storage_cost);

        Ok(total_storage_cost)
    }

    async fn dry_run_ptb(
        &self,
        programmable_transaction: ProgrammableTransaction,
        gas_coin: ObjectRef,
    ) -> Result<DryRunTransactionBlockResponse> {
        let gas_price = self.wallet.get_reference_gas_price().await?;
        let transaction = TransactionData::new_programmable(
            self.active_address()?,
            vec![gas_coin],
            programmable_transaction,
            self.config.gas_budget(),
            gas_price,
        );
        // Await the future to get the SuiClient.
        let client = self.wallet.get_client().await?;

        // Now you can call read_api() on the actual client.
        let response = client
            .read_api()
            .dry_run_transaction_block(transaction)
            .await?;

        // Return the response wrapped in Ok.
        Ok(response)
    }

    async fn sui_client(&self) -> Result<SuiClient> {
        self.wallet.get_client().await
    }

    // TODO(giac): This is a copy of `[WalletContext::active_address`] that works without borrowing
    //     as mutable. Use the implementation in `WalletContext` when the TODO there is fixed.
    pub(crate) fn active_address(&self) -> Result<SuiAddress> {
        if self.wallet.config.keystore.addresses().is_empty() {
            return Err(anyhow!(
                "No managed addresses. Create new address with `new-address` command."
            ));
        }

        // Ok to unwrap because we checked that config addresses not empty
        // Set it if not exists
        Ok(self
            .wallet
            .config
            .active_address
            .unwrap_or(*self.wallet.config.keystore.addresses().first().unwrap()))
    }

    /// Returns the [`ObjectRef`] of an arbitrary gas coin owned by the active wallet
    /// with a sufficient balance for the gas budget specified in the config.
    async fn gas_coin_ref(&self) -> Result<ObjectRef> {
        Ok(self
            .wallet
            .gas_for_owner_budget(
                self.active_address()?,
                self.config.gas_budget(),
                BTreeSet::new(),
            )
            .await?
            .1
            .object_ref())
    }

    /// Returns whether the site needs to be transferred to the active address.
    ///
    /// A new site needs to be transferred to the active address.
    fn needs_transfer(&self) -> bool {
        matches!(self.site_id, SiteIdentifier::NewSite(_))
    }
}

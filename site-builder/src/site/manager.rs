// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{collections::BTreeSet, str::FromStr, time::Duration};

use anyhow::{anyhow, Result};
use sui_keys::keystore::AccountKeystore;
use sui_sdk::{rpc_types::SuiTransactionBlockResponse, wallet_context::WalletContext};
use sui_types::{
    base_types::{ObjectID, ObjectRef, SuiAddress},
    transaction::{CallArg, ProgrammableTransaction},
    Identifier,
};

use super::{
    builder::SitePtb,
    resource::{Resource, ResourceOp},
    RemoteSiteFactory,
    SiteData,
    SiteDataDiff,
    SITE_MODULE,
};
use crate::{
    backoff::ExponentialBackoffConfig,
    display,
    publish::WhenWalrusUpload,
    retry_client::RetriableSuiClient,
    summary::SiteDataDiffSummary,
    util::{get_site_id_from_response, sign_and_send_ptb},
    walrus::Walrus,
    Config,
    EpochCountOrMax,
};

const MAX_RESOURCES_PER_PTB: usize = 200;
const OS_ERROR_DELAY: Duration = Duration::from_secs(1);
const MAX_RETRIES: u32 = 10;

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
    pub epochs: EpochCountOrMax,
    pub when_upload: WhenWalrusUpload,
    pub backoff_config: ExponentialBackoffConfig,
}

impl SiteManager {
    /// Creates a new site manager.
    pub async fn new(
        config: Config,
        site_id: SiteIdentifier,
        epochs: EpochCountOrMax,
        when_upload: WhenWalrusUpload,
    ) -> Result<Self> {
        Ok(SiteManager {
            walrus: config.walrus_client(),
            wallet: config.wallet()?,
            config,
            site_id,
            epochs,
            when_upload,
            // TODO(giac): This should be configurable.
            backoff_config: ExponentialBackoffConfig::default(),
        })
    }

    /// Updates the site with the given [`Resource`].
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

        if !walrus_updates.is_empty() {
            self.publish_to_walrus(&walrus_updates).await?;
        }

        let result = if site_updates.has_updates() {
            display::action("Updating the Walrus Site object on Sui");
            let result = self.execute_sui_updates(&site_updates).await?;
            display::done();
            result
        } else {
            SuiTransactionBlockResponse::default()
        };
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

            // Retry if the store operation fails with an os error.
            // NOTE(giac): This can be improved when the rust sdk for the client is open sourced.
            let mut retry_num = 0;
            loop {
                anyhow::ensure!(
                    retry_num < MAX_RETRIES,
                    "maximum number of retries exceeded"
                );
                tracing::debug!(retry_num, "attempting to store resource");
                retry_num += 1;
                let result = self
                    .walrus
                    .store(resource.full_path.clone(), self.epochs.clone(), false)
                    .await;

                match result {
                    Ok(_) => {
                        display::done();
                        break;
                    }
                    Err(err) => {
                        tracing::warn!(?err, "store operation failed");
                        if !err.to_string().contains("os error 54") || retry_num >= MAX_RETRIES {
                            return Err(err);
                        } else if retry_num >= MAX_RETRIES {
                            anyhow::bail!(
                                "a network error occurred when calling the Walrus CLI, \
                                and the maximum number of retries was exceeded"
                            );
                        } else {
                            tracing::warn!(
                                delay = ?OS_ERROR_DELAY,
                                "calling the Walrus CLI encountered an OS network error, retrying"
                            );
                            tokio::time::sleep(OS_ERROR_DELAY).await;
                        }
                    }
                }
            }
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
        let mut end = MAX_RESOURCES_PER_PTB.min(updates.resource_ops.len());
        tracing::debug!(
            total_ops = updates.resource_ops.len(),
            end,
            "preparing and committing the first PTB"
        );

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

        // Keep iterating to load resources.
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

    /// Adds a single resource to the site
    pub async fn update_single_resource(&mut self, resource: Resource) -> Result<()> {
        let ptb = SitePtb::new(
            self.config.package,
            Identifier::from_str(SITE_MODULE).expect("the str provided is valid"),
        )?;

        let SiteIdentifier::ExistingSite(site_id) = &self.site_id else {
            anyhow::bail!("`add_single_resource` is only supported for existing sites");
        };
        let mut ptb = ptb.with_call_arg(&self.wallet.get_object_ref(*site_id).await?.into())?;

        // First remove, then add the resource.
        let operations = vec![
            ResourceOp::Deleted(&resource),
            ResourceOp::Created(&resource),
        ];

        // Upload to Walrus
        tracing::debug!("uploading the resource to Walrus");
        let walrus_ops = operations
            .iter()
            .filter(|u| u.is_walrus_update(&self.when_upload))
            .collect::<Vec<_>>();
        self.publish_to_walrus(&walrus_ops).await?;

        // Create the PTB
        tracing::debug!("modifying the site object on chain");
        ptb.add_resource_operations(&operations)?;
        self.sign_and_send_ptb(ptb.finish(), self.gas_coin_ref().await?)
            .await?;
        Ok(())
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

    async fn sui_client(&self) -> Result<RetriableSuiClient> {
        RetriableSuiClient::new_from_wallet(&self.wallet, self.backoff_config.clone()).await
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

// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::{btree_map, BTreeSet, HashSet},
    num::NonZeroUsize,
    str::FromStr,
    time::Duration,
};

use anyhow::{anyhow, bail, Error, Result};
use sui_keys::keystore::AccountKeystore;
use sui_sdk::{
    rpc_types::{
        SuiExecutionStatus,
        SuiTransactionBlockEffectsAPI as _,
        SuiTransactionBlockResponse,
    },
    wallet_context::WalletContext,
};
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
    args::WalrusStoreOptions,
    backoff::ExponentialBackoffConfig,
    config::Config,
    display,
    publish::BlobManagementOptions,
    retry_client::RetriableSuiClient,
    site::builder::{SitePtbBuilderResultExt, PTB_MAX_MOVE_CALLS},
    summary::SiteDataDiffSummary,
    types::{Metadata, MetadataOp, RouteOps, SiteNameOp},
    util::{get_site_id_from_response, sign_and_send_ptb},
    walrus::{types::BlobId, Walrus},
};

const OS_ERROR_DELAY: Duration = Duration::from_secs(1);
const MAX_RETRIES: u32 = 10;

pub struct SiteManager {
    pub config: Config,
    pub walrus: Walrus,
    pub wallet: WalletContext,
    pub site_id: Option<ObjectID>,
    pub blob_options: BlobManagementOptions,
    pub backoff_config: ExponentialBackoffConfig,
    pub metadata: Option<Metadata>,
    pub site_name: Option<String>,
    pub walrus_options: WalrusStoreOptions,
    pub max_parallel_stores: NonZeroUsize,
}

impl SiteManager {
    /// Creates a new site manager.
    pub async fn new(
        config: Config,
        site_id: Option<ObjectID>,
        blob_options: BlobManagementOptions,
        walrus_options: WalrusStoreOptions,
        metadata: Option<Metadata>,
        site_name: Option<String>,
        max_parallel_stores: NonZeroUsize,
    ) -> Result<Self> {
        Ok(SiteManager {
            walrus: config.walrus_client(),
            wallet: config.load_wallet()?,
            config,
            site_id,
            blob_options,
            backoff_config: ExponentialBackoffConfig::default(),
            metadata,
            site_name,
            walrus_options,
            max_parallel_stores,
        })
    }

    /// Perform a dry-run of Walrus store operations for the given updates
    /// and return the total storage cost that would be incurred.
    async fn dry_run_walrus_single_blob_store(
        &mut self,
        walrus_updates: &Vec<&ResourceOp<'_>>,
    ) -> anyhow::Result<u64> {
        tracing::info!("Dry-running Walrus store operations");
        let mut total_storage_cost = 0;

        for update in walrus_updates {
            let resource = update.inner();
            let dry_run_outputs = self
                .walrus
                .dry_run_store(
                    resource.full_path.clone(),
                    self.walrus_options.epoch_arg.clone(),
                    !self.walrus_options.permanent,
                    false,
                )
                .await?;

            for dry_run_output in dry_run_outputs {
                total_storage_cost += dry_run_output.storage_cost;
            }
        }

        Ok(total_storage_cost)
    }

    /// Updates the site with the given [`Resource`].
    ///
    /// If the site does not exist, it is created and updated. The resources that need to be updated
    /// or created are published to Walrus.
    pub async fn update_site(
        &mut self,
        local_site_data: &SiteData,
        // Currently Quilts implementation, needs to store Quilt in advance, in order to get the
        // full resource needed to save on Sui. We use this to skip storing also as blobs.
        using_quilts: bool,
    ) -> Result<(SuiTransactionBlockResponse, SiteDataDiffSummary)> {
        tracing::debug!(?self.site_id, "creating or updating site");
        let retriable_client = self.sui_client().await?;
        let existing_site = match &self.site_id {
            Some(site_id) => {
                RemoteSiteFactory::new(&retriable_client, self.config.package)
                    .await?
                    .get_from_chain(*site_id)
                    .await?
            }
            None => SiteData::empty(),
        };
        tracing::debug!(?existing_site, "checked existing site");

        let site_updates = local_site_data.diff(&existing_site);

        let store_blobs = !using_quilts;
        if store_blobs {
            let walrus_candidate_set = if self.blob_options.is_check_extend() {
                // We need to check the status of all blobs: Return the full list of existing and added
                // blobs as possible updates.
                existing_site.replace_all(local_site_data)
            } else {
                // We only need to upload the new blobs.
                site_updates.clone()
            };
            // IMPORTANT: Perform the store operations on Walrus first, to ensure zero "downtime".
            self.select_and_store_single_blob_resources_to_walrus(&walrus_candidate_set)
                .await?;
        }

        // Check if there are any updates to the site on-chain.
        let result = if site_updates.has_updates() {
            display::action("Applying the Walrus Site object updates on Sui");
            self.execute_sui_updates(&site_updates)
                .await
                .inspect(|_| display::done())?
        } else {
            SuiTransactionBlockResponse::default()
        };

        // Extract the BlobIDs from deleted resources for Walrus cleanup
        let blobs_to_delete: Vec<BlobId> = collect_deletable_blob_candidates(&site_updates);

        if !blobs_to_delete.is_empty() {
            self.delete_from_walrus(&blobs_to_delete).await?;
        }

        Ok((result, site_updates.summary(&self.blob_options)))
    }

    /// Selects the necessary walrus store operations and executes them.
    async fn select_and_store_single_blob_resources_to_walrus(
        &mut self,
        walrus_candidate_set: &SiteDataDiff<'_>,
    ) -> Result<()> {
        let walrus_updates = walrus_candidate_set.get_walrus_updates(&self.blob_options);

        if !walrus_updates.is_empty() {
            if self.walrus_options.dry_run {
                let total_storage_cost = self
                    .dry_run_walrus_single_blob_store(&walrus_updates)
                    .await?;
                // Before doing the actual execution, perform a dry run
                display::action(format!(
                    "Estimated Storage Cost for this publish/update (Gas Cost Excluded): {total_storage_cost} FROST"
                ));

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
            }
            self.store_single_blob_resources_to_walrus(&walrus_updates)
                .await?;
        }
        Ok(())
    }

    /// Publishes the resources to Walrus.
    async fn store_single_blob_resources_to_walrus(
        &mut self,
        walrus_updates: &[&ResourceOp<'_>],
    ) -> Result<()> {
        for (idx, update_set) in walrus_updates
            .chunks(self.max_parallel_stores.get())
            .enumerate()
        {
            display::action(format!(
                "Storing resources on Walrus: batch {} of {}",
                idx + 1,
                walrus_updates
                    .len()
                    .div_ceil(self.max_parallel_stores.get()),
            ));
            self.store_multiple_to_walrus_with_retry(update_set).await?;
            display::done();
        }
        Ok(())
    }

    async fn store_multiple_to_walrus_with_retry(
        &mut self,
        update_batch: &[&ResourceOp<'_>],
    ) -> Result<()> {
        let deletable = !self.walrus_options.permanent;
        let resource_paths = update_batch
            .iter()
            .map(|update| update.inner().full_path.clone())
            .collect::<Vec<_>>();

        tracing::debug!(?resource_paths, "storing resource batch on Walrus",);

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
                .store(
                    resource_paths.clone(),
                    self.walrus_options.epoch_arg.clone(),
                    false,
                    deletable,
                )
                .await;

            match result {
                Ok(_) => {
                    break;
                }
                Err(error) => {
                    if !is_retriable_error(&error) || retry_num >= MAX_RETRIES {
                        return Err(error);
                    } else {
                        tracing::warn!(
                            ?error,
                            delay = ?OS_ERROR_DELAY,
                            "calling the Walrus CLI encountered a retriable error, retrying"
                        );
                        tokio::time::sleep(OS_ERROR_DELAY).await;
                    }
                }
            }
        }
        Ok(())
    }

    /// Deletes the resources from Walrus.
    pub async fn delete_from_walrus(&mut self, blob_ids: &[BlobId]) -> Result<()> {
        tracing::debug!(?blob_ids, "deleting blob from Walrus");
        display::action("Running the delete commands on Walrus");
        let output = self.walrus.delete(blob_ids).await?;
        display::done();

        for blob_output in output {
            if let Some(blob_id) = blob_output.blob_identity.blob_id {
                if blob_ids.contains(&blob_id) {
                    tracing::debug!(%blob_id, "blob deleted successfully");
                } else {
                    display::error(format!(
                        "Could not delete blob {blob_id}, may be already deleted or may be a permanent blob"
                    ));
                }
            } else {
                tracing::error!(?blob_output.blob_identity, "the blob ID is missing from the identity");
            }
        }

        Ok(())
    }

    /// Executes the updates on Sui.
    async fn execute_sui_updates(
        &self,
        updates: &SiteDataDiff<'_>,
    ) -> Result<SuiTransactionBlockResponse> {
        tracing::debug!(
            address=?self.active_address()?,
            ?updates,
            "starting to update site resources on chain",
        );

        // 1st iteration
        // Keep 3 operations for optional route deletion + creation + site-transfer
        let ptb = SitePtb::<(), 1021>::new(
            self.config.package,
            Identifier::from_str(SITE_MODULE).expect("the str provided is valid"),
        );

        // Add the call arg if we are updating a site, or add the command to create a new site.
        // Keep 3 operations for optional route deletion + creation + site-transfer
        let mut ptb = match &self.site_id {
            Some(site_id) => {
                let ptb = ptb.with_call_arg(&self.wallet.get_object_ref(*site_id).await?.into())?;
                // Also update metadata if there is a diff
                match updates.metadata_op {
                    MetadataOp::Update => {
                        ptb.with_update_metadata(self.metadata.clone().unwrap_or_default())?
                    }
                    MetadataOp::Noop => ptb,
                }
            }
            None => ptb.with_create_site(
                self.site_name.as_deref().unwrap_or("My Walrus Site"),
                self.metadata.clone(),
            )?,
        };

        if let (Some(site_name), SiteNameOp::Update) = (&self.site_name, updates.site_name_op) {
            ptb.update_name(site_name)?;
        }

        let mut resources_iter = updates.resource_ops.iter().peekable();
        ptb.add_resource_operations(&mut resources_iter)
            .ok_if_limit_reached()?;

        // Update ptb limit to add routes. Keep 1 operation for transfer.
        let mut ptb = ptb.with_max_move_calls::<1023>();

        let mut routes_iter = btree_map::Iter::default().peekable();
        if let RouteOps::Replace(new_routes) = &updates.route_ops {
            if new_routes.is_empty() {
                ptb.remove_routes()
            } else {
                ptb.replace_routes()
            }?;
            routes_iter = new_routes.0.iter().peekable();
        }

        ptb.add_route_operations(&mut routes_iter)
            .ok_if_limit_reached()?;

        if self.needs_transfer() {
            ptb = ptb.with_max_move_calls(); // Update to actual max.
            ptb.transfer_site(self.active_address()?)?;
        }

        let retry_client = self.sui_client().await?;
        let result = self
            .sign_and_send_ptb(ptb.finish(), self.gas_coin_ref().await?, &retry_client)
            .await?;

        // Check explicitly for execution failures.
        if let Some(SuiExecutionStatus::Failure { error }) =
            result.effects.as_ref().map(|e| e.status())
        {
            bail!(
                "site ptb failed with error: {error} [tx_digest={}]",
                result.digest
            );
        }

        let site_object_id = match &self.site_id {
            Some(site_id) => *site_id,
            None => {
                let resp = result
                    .effects
                    .as_ref()
                    .ok_or(anyhow!("the result did not have effects"))?;
                get_site_id_from_response(self.active_address()?, resp)?
            }
        };

        // Keep iterating to load all resources and routes.
        while resources_iter.peek().is_some() || routes_iter.peek().is_some() {
            let ptb: SitePtb<(), { PTB_MAX_MOVE_CALLS }> = SitePtb::new(
                self.config.package,
                Identifier::from_str(SITE_MODULE).expect("the str provided is valid"),
            );
            let call_arg: CallArg = self.wallet.get_object_ref(site_object_id).await?.into();
            let mut ptb = ptb.with_call_arg(&call_arg)?;

            ptb.add_resource_operations(&mut resources_iter)
                .ok_if_limit_reached()?;
            ptb.add_route_operations(&mut routes_iter)
                .ok_if_limit_reached()?;

            let resource_result = self
                .sign_and_send_ptb(ptb.finish(), self.gas_coin_ref().await?, &retry_client)
                .await?;
            if let Some(SuiExecutionStatus::Failure { error }) =
                resource_result.effects.as_ref().map(|e| e.status())
            {
                anyhow::bail!(
                    "resource ptb failed with error: {error} [tx_digest={}]",
                    resource_result.digest
                );
            }
        }

        Ok(result)
    }

    /// Adds a single resource to the site
    pub async fn update_single_resource(&mut self, resource: Resource) -> Result<()> {
        let ptb = SitePtb::<(), 1021>::new(
            self.config.package,
            Identifier::from_str(SITE_MODULE).expect("the str provided is valid"),
        );

        let Some(site_id) = &self.site_id else {
            anyhow::bail!("`add_single_resource` is only supported for existing sites");
        };
        let mut ptb = ptb.with_call_arg(&self.wallet.get_object_ref(*site_id).await?.into())?;

        // First remove, then add the resource.
        let operations = [
            ResourceOp::Deleted(&resource),
            ResourceOp::Created(&resource),
        ];

        // Upload to Walrus
        tracing::debug!("uploading the resource to Walrus");
        let walrus_ops = operations
            .iter()
            .filter(|u| u.is_walrus_update(&self.blob_options))
            .collect::<Vec<_>>();

        //Perform dry run
        if self.walrus_options.dry_run {
            let total_storage_cost = self.dry_run_walrus_single_blob_store(&walrus_ops).await?;
            // Before doing the actual execution, perform a dry run
            display::action(format!(
                "Estimated Storage Cost for this publish/update (Gas Cost Excluded): {total_storage_cost} FROST"
            ));
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
        }
        self.store_single_blob_resources_to_walrus(&walrus_ops)
            .await?;

        // Create the PTB
        tracing::debug!("modifying the site object on chain");
        ptb.add_resource_operations(&mut operations.iter().peekable())?;
        self.sign_and_send_ptb(
            ptb.finish(),
            self.gas_coin_ref().await?,
            &self.sui_client().await?,
        )
        .await?;
        Ok(())
    }

    async fn sign_and_send_ptb(
        &self,
        programmable_transaction: ProgrammableTransaction,
        gas_coin: ObjectRef,
        retry_client: &RetriableSuiClient,
    ) -> Result<SuiTransactionBlockResponse> {
        sign_and_send_ptb(
            self.active_address()?,
            &self.wallet,
            retry_client,
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
        self.site_id.is_none()
    }
}

fn is_retriable_error(error: &Error) -> bool {
    let error_message = error.to_string();
    if error_message.contains("os error 54") {
        // The connection was reset by the peer -- a common RPC error under load.
        true
    } else if error_message.contains("response does not contain object data") {
        // The RPC may be slow, and does not have the correct object version.
        true
    } else {
        false
    }
}

/// Collects the `BlobId`s from the site_updates Deleted ResourceOps.
/// These are candidates for deletion from Walrus.
fn collect_deletable_blob_candidates(site_updates: &SiteDataDiff) -> Vec<BlobId> {
    site_updates
        .resource_ops
        .iter()
        .filter_map(|op| match op {
            ResourceOp::Deleted(resource) => Some(resource.info.blob_id),
            _ => None,
        })
        // Collect first to a hash-set to keep unique blob-ids.
        .collect::<HashSet<_>>()
        .into_iter()
        .collect()
}

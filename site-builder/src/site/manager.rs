// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::{btree_map, BTreeSet, HashSet},
    iter::Peekable,
    str::FromStr,
};

use anyhow::{anyhow, bail, Result};
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
    transaction::{CallArg, ProgrammableTransaction, TransactionData, TransactionDataAPI},
    Identifier,
};
use tracing::warn;

use super::{
    builder::SitePtb,
    resource::ResourceOp,
    RemoteSiteFactory,
    SiteData,
    SiteDataDiff,
    SITE_MODULE,
};
use crate::{
    backoff::ExponentialBackoffConfig,
    config::Config,
    display,
    retry_client::RetriableSuiClient,
    site::{
        builder::{SitePtbBuilderResultExt, PTB_MAX_MOVE_CALLS},
        resource::ResourceSet,
    },
    summary::SiteDataDiffSummary,
    types::{Metadata, MetadataOp, ObjectCache, RouteOps, SiteNameOp},
    util::{get_site_id_from_response, get_site_object_via_graphql, sign_and_send_ptb},
    walrus::{types::BlobId, Walrus},
};

#[cfg(test)]
#[path = "../unit_tests/site.manager.tests.rs"]
mod manager_tests;

pub struct SiteManager {
    pub config: Config,
    pub walrus: Walrus,
    pub wallet: WalletContext,
    pub site_id: Option<ObjectID>,
    pub backoff_config: ExponentialBackoffConfig,
    pub metadata: Option<Metadata>,
    pub site_name: Option<String>,
    pub object_cache: ObjectCache,
}

impl SiteManager {
    /// Creates a new site manager.
    pub async fn new(
        config: Config,
        site_id: Option<ObjectID>,
        metadata: Option<Metadata>,
        site_name: Option<String>,
    ) -> Result<Self> {
        Ok(SiteManager {
            walrus: config.walrus_client(),
            wallet: config.load_wallet()?,
            config,
            site_id,
            backoff_config: ExponentialBackoffConfig::default(),
            metadata,
            site_name,
            object_cache: ObjectCache::new(),
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

        // Check if there are any updates to the site on-chain.
        let result = if site_updates.has_updates() {
            println!(); // Empty line before applying action for consistency
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

        Ok((result, site_updates.summary()))
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

    /// Builds the initial PTB for site creation/update with initial resources
    async fn build_initial_ptb<'a>(
        &self,
        updates: &'a SiteDataDiff<'_>,
    ) -> Result<(
        ProgrammableTransaction,
        Peekable<std::slice::Iter<'a, ResourceOp<'a>>>,
        Peekable<btree_map::Iter<'a, String, String>>,
    )> {
        // 1st iteration
        // Keep 4 operations for optional update_name + route deletion + creation + site-transfer
        const INITIAL_MAX: u16 = PTB_MAX_MOVE_CALLS - 4;
        let ptb = SitePtb::<(), INITIAL_MAX>::new(
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
        const TRANSFER_MAX: u16 = PTB_MAX_MOVE_CALLS - 1;
        let mut ptb = ptb.with_max_move_calls::<TRANSFER_MAX>();

        let mut routes_iter = btree_map::Iter::default().peekable();
        // TODO: Could this logic be transferred inside `SitePtb`?
        if let RouteOps::Replace(new_routes) = &updates.route_ops {
            if new_routes.is_empty() {
                ptb.remove_routes()
            } else {
                ptb.replace_routes()
            }?;
            routes_iter = new_routes.0.iter().peekable();
        }

        // Add routes only if all resources have been added.
        if resources_iter.peek().is_none() {
            ptb.add_route_operations(&mut routes_iter)
                .ok_if_limit_reached()?;
        }

        let mut ptb = ptb.with_max_move_calls::<PTB_MAX_MOVE_CALLS>(); // Update to actual max.
        if self.needs_transfer() {
            ptb.transfer_site(self.active_address()?)?;
        }

        Ok((ptb.finish(), resources_iter, routes_iter))
    }

    /// Builds PTBs for remaining resources and routes
    async fn build_remaining_resources_ptbs<'a>(
        &self,
        site_object_id: ObjectID,
        mut resources_iter: Peekable<std::slice::Iter<'a, ResourceOp<'a>>>,
        mut routes_iter: Peekable<btree_map::Iter<'a, String, String>>,
    ) -> Result<Vec<ProgrammableTransaction>> {
        let mut transactions = Vec::new();
        
        while resources_iter.peek().is_some() || routes_iter.peek().is_some() {
            let ptb: SitePtb<(), { PTB_MAX_MOVE_CALLS }> = SitePtb::new(
                self.config.package,
                Identifier::from_str(SITE_MODULE).expect("the str provided is valid"),
            );
            let site_object_ref = self.wallet.get_object_ref(site_object_id).await?;
            let call_arg: CallArg = site_object_ref.into();
            let mut ptb = ptb.with_call_arg(&call_arg)?;

            ptb.add_resource_operations(&mut resources_iter)
                .ok_if_limit_reached()?;

            // Add routes only if all resources have been added.
            if resources_iter.peek().is_none() {
                ptb.add_route_operations(&mut routes_iter)
                    .ok_if_limit_reached()?;
            }

            transactions.push(ptb.finish());
        }
        
        Ok(transactions)
    }

    /// Estimates Sui gas costs for site updates by dry-running transactions
    pub async fn estimate_sui_gas(&mut self, local_site_data: &SiteData) -> Result<u64> {
        tracing::debug!(
            address=?self.active_address()?,
            "estimating Sui gas for site updates",
        );

        let retry_client = self.sui_client().await?;
        
        // Get existing site data (if any)
        let existing_site = match &self.site_id {
            Some(site_id) => {
                RemoteSiteFactory::new(&retry_client, self.config.package)
                    .await?
                    .get_from_chain(*site_id)
                    .await?
            }
            None => SiteData::empty(),
        };

        // Calculate the diff between current and new site data
        let updates = local_site_data.diff(&existing_site);

        // Build the initial PTB
        let (initial_ptb, mut resources_iter, mut routes_iter) = self.build_initial_ptb(&updates).await?;

        let gas_ref = self.gas_coin_ref().await?;
        
        // Dry run initial PTB
        let initial_response = self.dry_run_ptb(initial_ptb.clone(), gas_ref, &retry_client, false).await?;
        let initial_gas = initial_response.effects.gas_cost_summary().net_gas_usage() as u64;
        
        println!(
            "Initial PTB gas cost: {} MIST ({:.2} SUI) ({} commands)",
            initial_gas,
            initial_gas as f64 / 1_000_000_000.0,
            initial_ptb.commands.len()
        );

        // Check if we'll need additional PTBs by peeking at the iterators
        let has_remaining_resources = resources_iter.peek().is_some() || routes_iter.peek().is_some();
        
        let site_object_id = if !has_remaining_resources {
            // No additional PTBs needed - just return the initial PTB gas cost
            println!(
                "Total estimated gas cost: {} MIST ({:.2} SUI)",
                initial_gas,
                initial_gas as f64 / 1_000_000_000.0
            );
            return Ok(initial_gas);
        } else {
            // Additional PTBs needed - get site object ID
            // Since their validation will depend on it
            match &self.site_id {
                Some(id) => *id, // Use existing site ID
                None => {
                    // For new sites, query for existing site object
                    // We can use any one but present on the chain
                    let existing_site_id = get_site_object_via_graphql(&self.wallet).await;
                    if let Some(existing_id) = existing_site_id {
                        existing_id
                    } else {
                        return Err(anyhow::anyhow!("No existing site object found for gas estimation"));
                    }
                }
            }
        };

        // Build remaining PTBs
        let remaining_ptbs = self.build_remaining_resources_ptbs(
            site_object_id,
            resources_iter,
            routes_iter,
        ).await?;

        // Dry run remaining PTBs
        let mut total_gas = initial_gas;
        if remaining_ptbs.is_empty() {
            println!("Single transaction required for all updates");
        } else {
            println!(
                "Multiple transactions required: {} additional resource PTBs",
                remaining_ptbs.len()
            );
        }
        
        for (i, ptb) in remaining_ptbs.iter().enumerate() {
            let gas_ref = self.gas_coin_ref().await?;
            let response = self.dry_run_ptb(ptb.clone(), gas_ref, &retry_client, true).await?;
            
            // If dev_inspect failed, estimate gas based on command count
            let gas_cost = if response.error.is_some() {
                // Use heuristic: scale based on command count compared to initial PTB
                let command_ratio = ptb.commands.len() as f64 / initial_ptb.commands.len() as f64;
                (initial_gas as f64 * command_ratio * 0.8) as u64 // Resource PTBs are typically simpler
            } else {
                response.effects.gas_cost_summary().net_gas_usage() as u64
            };
            
            total_gas += gas_cost;
            
            // Debug: show if there were any errors in dev_inspect
            if let Some(ref error) = response.error {
                println!(
                    "Resource PTB {}/{} had dev_inspect error: {}",
                    i + 1,
                    remaining_ptbs.len(),
                    error
                );
                println!(
                    "Estimated cost based on {} commands", 
                    ptb.commands.len()
                );
            }
            
            println!(
                "Resource PTB {}/{}: {} MIST ({:.2} SUI) ({} commands)",
                i + 1,
                remaining_ptbs.len(),
                gas_cost,
                gas_cost as f64 / 1_000_000_000.0,
                ptb.commands.len()
            );
        }

        println!(
            "Total estimated gas cost: {} MIST ({:.2} SUI)",
            total_gas,
            total_gas as f64 / 1_000_000_000.0
        );

        Ok(total_gas)
    }

    /// Dry runs a PTB and returns the response
    async fn dry_run_ptb(
        &mut self,
        ptb: ProgrammableTransaction,
        gas_coin: ObjectRef,
        retry_client: &RetriableSuiClient,
        use_modified_for_estimation: bool,
    ) -> Result<sui_sdk::rpc_types::DevInspectResults> {
        // For new sites and resource PTBs, use modified PTB for estimation
        let estimation_ptb = if use_modified_for_estimation && self.site_id.is_none() {
            self.create_estimation_ptb(&ptb)
        } else {
            ptb
        };

        // Get the current reference gas price
        let gas_price = retry_client
            .client()
            .read_api()
            .get_reference_gas_price()
            .await?;

        let tx_data = TransactionData::new_programmable(
            self.wallet.active_address()?,
            vec![gas_coin],
            estimation_ptb,
            self.config.gas_budget(),
            gas_price, // Use actual reference gas price
        );

        // It makes sense to use dev_inspect_transaction_block instead of dry_run_transaction_block 
        // because the latter requires extended validations like 
        // real object ids present on the chain for quilts and signed transactions.
        // For our use case we actually need only high level verification
        // and gas cost estimation.
        let response = retry_client
            .client()
            .read_api()
            .dev_inspect_transaction_block(
                self.wallet.active_address()?,
                tx_data.into_kind(),
                Some(gas_price.into()),
                None, // epoch
                Some(sui_sdk::rpc_types::DevInspectArgs {
                    skip_checks: Some(true), // Skip validation checks
                    gas_sponsor: None,
                    gas_budget: None,
                    gas_objects: None,
                    show_raw_txn_data_and_effects: None,
                }),
            )
            .await?;

        Ok(response)
    }

    /// Creates a modified PTB for estimation - currently just returns the original
    fn create_estimation_ptb(&self, ptb: &ProgrammableTransaction) -> ProgrammableTransaction {
        // For now, return the original PTB unchanged
        // We'll rely on using a real object ID of the right type
        ptb.clone()
    }

    /// Executes the updates on Sui.
    async fn execute_sui_updates(
        &mut self,
        updates: &SiteDataDiff<'_>,
    ) -> Result<SuiTransactionBlockResponse> {
        tracing::debug!(
            address=?self.active_address()?,
            ?updates,
            "starting to update site resources on chain",
        );

        // Build the initial PTB
        let (initial_ptb, resources_iter, routes_iter) = self.build_initial_ptb(updates).await?;

        let retry_client = self.sui_client().await?;
        assert!(initial_ptb.commands.len() <= PTB_MAX_MOVE_CALLS as usize);
        
        // TODO: #SEW-498 Verify gas_ref. Currently, we do not have the last tx the user submitted through
        // walrus.
        let gas_ref = self.gas_coin_ref().await?;
        let result = self
            .sign_and_send_ptb(initial_ptb, gas_ref, &retry_client)
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

        // Build remaining PTBs
        let remaining_ptbs = self.build_remaining_resources_ptbs(
            site_object_id,
            resources_iter,
            routes_iter,
        ).await?;

        // Execute remaining PTBs
        for ptb in remaining_ptbs {
            let gas_ref = self.gas_coin_ref().await?;
            let resource_result = self
                .sign_and_send_ptb(ptb, gas_ref, &retry_client)
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

    pub async fn update_resources(&mut self, resources: ResourceSet) -> Result<()> {
        let Some(site_id) = self.site_id else {
            anyhow::bail!("`update_resources` is only supported for existing sites");
        };

        // Create operations: for each resource, delete then immediately create it
        // This ensures the delete/create pairs are adjacent, which is better for updates
        let operations: Vec<_> = resources
            .inner
            .iter()
            .flat_map(|resource| [ResourceOp::Deleted(resource), ResourceOp::Created(resource)])
            .collect();

        let mut operations_iter = operations.iter().peekable();
        let retry_client = self.sui_client().await?;

        tracing::debug!("modifying the site object on chain");

        // Create PTBs until all operations are processed
        while operations_iter.peek().is_some() {
            let ptb = SitePtb::<(), PTB_MAX_MOVE_CALLS>::new(
                self.config.package,
                Identifier::from_str(SITE_MODULE).expect("the str provided is valid"),
            );
            let mut site_obj_ref = self.wallet.get_object_ref(site_id).await?;
            site_obj_ref = self.verify_object_ref_choose_latest(site_obj_ref)?;
            let call_arg: CallArg = site_obj_ref.into();
            let mut ptb = ptb.with_call_arg(&call_arg)?;

            ptb.add_resource_operations(&mut operations_iter)
                .ok_if_limit_reached()?;

            let gas_ref = self.gas_coin_ref().await?;
            self.sign_and_send_ptb(ptb.finish(), gas_ref, &retry_client)
                .await?;
        }

        Ok(())
    }

    async fn sign_and_send_ptb(
        &mut self,
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
            &mut self.object_cache,
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

    // TODO: Why require a **single** gas-coin and not do select_coins?
    /// Returns the [`ObjectRef`] of an arbitrary gas coin owned by the active wallet
    /// with a sufficient balance for the gas budget specified in the config.
    async fn gas_coin_ref(&mut self) -> Result<ObjectRef> {
        // Keep re-fetching the coin, until it matches the latest state stored by our cache, as
        // older versions might show more balance than its actual balance.
        let mut backoff = self.backoff_config.get_strategy(rand::random());
        loop {
            let gas_coin = self
                .wallet
                .gas_for_owner_budget(
                    self.active_address()?,
                    self.config.gas_budget(),
                    BTreeSet::new(),
                )
                .await?;

            let gas_obj_ref = gas_coin.1.object_ref();
            let latest = self.verify_object_ref_choose_latest(gas_obj_ref)?;
            if gas_obj_ref == latest {
                return Ok(latest);
            }

            // Fullnode returned stale version, wait and retry.
            if let Some(delay) = backoff.next() {
                warn!(
                    ?gas_obj_ref,
                    ?latest,
                    ?delay,
                    "fullnode returned stale gas coin version; retrying after delay"
                );
                tokio::time::sleep(delay).await;
            } else {
                bail!("fullnode returned stale gas coin version after max retries exhausted")
            }
        }
    }

    /// Returns whether the site needs to be transferred to the active address.
    ///
    /// A new site needs to be transferred to the active address.
    fn needs_transfer(&self) -> bool {
        self.site_id.is_none()
    }

    fn verify_object_ref_choose_latest(
        &mut self,
        object_ref: ObjectRef,
    ) -> anyhow::Result<ObjectRef> {
        let cached: Option<&ObjectRef> = self.object_cache.get(&object_ref.0);
        match cached {
            // TODO: #SEW-503 Will we have a problem if during the execute we use an FN with an
            // older version? Does RetriableSuiClient mitigate this?
            // If the cached version is bigger than the fetched, just used the cached.
            Some(&cached) if cached.1 > object_ref.1 => {
                warn!("Fullnode returned older object reference ({object_ref:?}) than its latest. Using latest cached ({cached:?}).");
                Ok(cached)
            }
            Some(&cached) if cached != object_ref => {
                // This should not happen as long as user is not executing transactions with this
                // wallet-address in parallel.
                bail!("Fullnode returned newer object version ({object_ref:?}) than the one cached ({cached:?}");
            }
            None => {
                self.object_cache.insert(object_ref.0, object_ref);
                Ok(object_ref)
            }
            _ => Ok(object_ref),
        }
    }
}

/// Collects the `BlobId`s from the site_updates Deleted ResourceOps.
/// These are candidates for deletion from Walrus.
/// Resources that have been deleted but also created are excluded.
fn collect_deletable_blob_candidates(site_updates: &SiteDataDiff) -> Vec<BlobId> {
    let mut deleted = site_updates
        .resource_ops
        .iter()
        .filter_map(|op| match op {
            ResourceOp::Deleted(resource) => Some(resource.info.blob_id),
            _ => None,
        })
        // Collect first to a hash-set to keep unique blob-ids.
        .collect::<HashSet<_>>();
    let resource_deleted_but_blob_extended = site_updates
        .resource_ops
        .iter()
        .filter_map(|op| match op {
            ResourceOp::Created(resource) if deleted.contains(&resource.info.blob_id) => {
                Some(resource.info.blob_id)
            }
            _ => None,
        })
        .collect::<HashSet<_>>();
    deleted.retain(|blob_id| !resource_deleted_but_blob_extended.contains(blob_id));
    deleted.into_iter().collect()
}

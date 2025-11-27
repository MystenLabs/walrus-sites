// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::{btree_map, BTreeSet, HashSet},
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
    transaction::{CallArg, ProgrammableTransaction},
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
    util::{get_site_id_from_response, sign_and_send_ptb},
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

        let retry_client = self.sui_client().await?;
        let built_ptb = ptb.finish();
        assert!(built_ptb.commands.len() <= PTB_MAX_MOVE_CALLS as usize);
        // TODO: Verify gas_ref. Currently, we do not have the last tx the user submitted through
        // walrus.
        let gas_ref = self.gas_coin_ref().await?;
        let result = self
            .sign_and_send_ptb(built_ptb, gas_ref, &retry_client)
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
            let mut site_object_ref = self.wallet.get_object_ref(site_object_id).await?;
            site_object_ref = self.verify_object_ref_choose_latest(site_object_ref)?;
            let call_arg: CallArg = site_object_ref.into();
            let mut ptb = ptb.with_call_arg(&call_arg)?;

            ptb.add_resource_operations(&mut resources_iter)
                .ok_if_limit_reached()?;

            // Add routes only if all resources have been added.
            if resources_iter.peek().is_none() {
                ptb.add_route_operations(&mut routes_iter)
                    .ok_if_limit_reached()?;
            }

            let gas_ref = self.gas_coin_ref().await?;
            let resource_result = self
                .sign_and_send_ptb(ptb.finish(), gas_ref, &retry_client)
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
        const MAX_RETRIES: usize = 10;
        for _ in 0..MAX_RETRIES {
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
        }
        bail!("Fullnode returned stale object version after {MAX_RETRIES} retries")
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
            // TODO: Will we have a problem if during the execute we use an FN with an older
            // version? Does RetriableSuiClient mitigate this?
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

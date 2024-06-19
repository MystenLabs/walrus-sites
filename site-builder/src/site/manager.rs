// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{collections::BTreeSet, str::FromStr};

use anyhow::{anyhow, Result};
use sui_keys::keystore::AccountKeystore;
use sui_sdk::{
    rpc_types::{SuiMoveValue, SuiObjectDataOptions, SuiTransactionBlockResponse},
    wallet_context::WalletContext,
    SuiClient,
};
use sui_types::{
    base_types::{ObjectID, ObjectRef, SuiAddress},
    transaction::{Argument, ProgrammableTransaction, TransactionData},
    Identifier,
};

use super::resource::{OperationsSummary, ResourceInfo, ResourceManager, ResourceOp, ResourceSet};
use crate::{
    display,
    site::builder::{SiteCall, SitePtb},
    util::{self, get_struct_from_object_response},
    walrus::Walrus,
    Config,
};

const SITE_MODULE: &str = "site";

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
    pub force: bool,
}

impl SiteManager {
    /// Creates a new site manager.
    pub async fn new(
        config: Config,
        walrus: Walrus,
        wallet: WalletContext,
        site_id: SiteIdentifier,
        epochs: u64,
        force: bool,
    ) -> Result<Self> {
        Ok(SiteManager {
            walrus,
            wallet,
            config,
            site_id,
            epochs,
            force,
        })
    }

    /// Updates the site with the given [`Resource`](super::resource::Resource).
    ///
    /// If the site does not exist, it is created and updated. The resources that need to be updated
    /// or created are published to Walrus.
    pub async fn update_site(
        &self,
        resources: &ResourceManager,
    ) -> Result<(SuiTransactionBlockResponse, OperationsSummary)> {
        let ptb = SitePtb::new(
            self.config.package,
            Identifier::from_str(SITE_MODULE).expect("the str provided is valid"),
        )?;
        let (ptb, existing_resources, needs_transfer) = match &self.site_id {
            SiteIdentifier::ExistingSite(site_id) => (
                ptb.with_call_arg(&self.wallet.get_object_ref(*site_id).await?.into())?,
                self.get_existing_resources(*site_id).await?,
                false,
            ),
            SiteIdentifier::NewSite(site_name) => (
                ptb.with_create_site(site_name)?,
                ResourceSet::default(),
                true,
            ),
        };
        tracing::debug!(?existing_resources, "checked existing resources");
        let update_operations = if self.force {
            existing_resources.replace_all(&resources.resources)
        } else {
            resources.resources.diff(&existing_resources)
        };
        tracing::debug!(operations=?update_operations, "list of operations computed");

        self.publish_to_walrus(&update_operations).await?;

        if !update_operations.is_empty() {
            display::action("Updating the Walrus Site object on Sui");
            let result = self
                .execute_updates(ptb, &update_operations, needs_transfer)
                .await?;
            display::done();
            return Ok((result, update_operations.into()));
        }
        // TODO(giac) improve this return
        Ok((
            SuiTransactionBlockResponse::default(),
            update_operations.into(),
        ))
    }

    /// Publishes the resources to Walrus.
    async fn publish_to_walrus<'b>(&self, updates: &[ResourceOp<'b>]) -> Result<()> {
        let to_update = updates
            .iter()
            .filter(|u| matches!(u, ResourceOp::Created(_)))
            .collect::<Vec<_>>();
        tracing::debug!(resources=?to_update, "publishing new or updated resources to Walrus");

        for update in to_update.iter() {
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
                .store(resource.full_path.clone(), self.epochs, self.force)?;
            display::done();
        }
        Ok(())
    }

    async fn execute_updates<'b>(
        &self,
        mut ptb: SitePtb<Argument>,
        updates: &[ResourceOp<'b>],
        transfer: bool,
    ) -> Result<SuiTransactionBlockResponse> {
        tracing::debug!(
            address=?self.active_address()?,
            "starting to update site resources on chain",
        );
        ptb.add_calls(
            updates
                .iter()
                .map(SiteCall::try_from)
                .collect::<Result<Vec<_>>>()?,
        )?;
        if transfer {
            ptb.transfer_arg(self.active_address()?, ptb.site_argument());
        }
        self.sign_and_send_ptb(ptb.finish(), self.gas_coin_ref().await?)
            .await
    }

    async fn sign_and_send_ptb(
        &self,
        programmable_transaction: ProgrammableTransaction,
        gas_coin: ObjectRef,
    ) -> Result<SuiTransactionBlockResponse> {
        let gas_price = self.wallet.get_reference_gas_price().await?;
        let transaction = TransactionData::new_programmable(
            self.active_address()?,
            vec![gas_coin],
            programmable_transaction,
            self.config.gas_budget(),
            gas_price,
        );
        let transaction = self.wallet.sign_transaction(&transaction);
        self.wallet.execute_transaction_may_fail(transaction).await
    }

    async fn get_existing_resources(&self, site_id: ObjectID) -> Result<ResourceSet> {
        let resource_ids = self.get_existing_resource_ids(site_id).await?;
        let resources = futures::future::try_join_all(
            resource_ids
                .into_iter()
                .map(|id| self.get_remote_resource_info(id)),
        )
        .await?;
        Ok(ResourceSet::from_iter(resources))
    }

    /// Get the resources already published to the site
    async fn get_existing_resource_ids(&self, site_id: ObjectID) -> Result<Vec<ObjectID>> {
        Ok(
            util::get_existing_resource_ids(&self.sui_client().await?, site_id)
                .await?
                .into_values()
                .collect(),
        )
    }

    /// Get the resource that is hosted on chain at the given object ID
    async fn get_remote_resource_info(&self, object_id: ObjectID) -> Result<ResourceInfo> {
        let object = get_struct_from_object_response(
            &self
                .sui_client()
                .await?
                .read_api()
                .get_object_with_options(object_id, SuiObjectDataOptions::new().with_content())
                .await?,
        )?;
        get_dynamic_field!(object, "value", SuiMoveValue::Struct)?.try_into()
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
}

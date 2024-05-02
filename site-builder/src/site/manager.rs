use std::collections::HashSet;

use anyhow::{anyhow, Result};
use sui_sdk::{
    rpc_types::{SuiMoveValue, SuiObjectDataOptions, SuiTransactionBlockResponse},
    SuiClient,
};
use sui_types::base_types::ObjectID;

use super::{
    builder::{BlocksiteCall, BlocksitePtb},
    resource::ResourceManager,
};
use crate::{
    site::resource::Resource,
    util::{
        get_existing_resource_ids, get_object_ref_from_id, get_site_id_from_response,
        get_struct_from_object_response, sign_and_send_ptb,
    },
    Config,
};

pub const MAX_TX_SIZE: usize = 131_072;
pub const MAX_ARG_SIZE: usize = 16_300;
pub const MAX_OBJ_SIZE: usize = 256_000;
pub const ARG_MARGIN: usize = 1_000; // TODO: check if there is better way
pub const TX_MARGIN: usize = 10_000; // TODO: check if there is better way
pub const OBJ_MARGIN: usize = 10_000; // TODO: check if there is better way

pub struct SiteManager<'a> {
    pub client: SuiClient,
    pub config: &'a Config,
    pub site_id: Option<ObjectID>,
}

impl<'a> SiteManager<'a> {
    pub async fn new(site_id: Option<ObjectID>, config: &'a Config) -> Result<Self> {
        let client = config.network.get_sui_client().await?;
        Ok(SiteManager {
            client,
            config,
            site_id,
        })
    }

    /// Update the site with the given [Resource]s
    /// If the site does not exist, it is created and updated.
    pub async fn update_site(
        &mut self,
        site_name: &str,
        resources: &mut ResourceManager,
    ) -> Result<Vec<SuiTransactionBlockResponse>> {
        let to_delete = self.diff_site(resources).await?;
        tracing::info!("Resources to delete: {:?}", to_delete);
        tracing::info!("To add: {}", resources);
        let calls = self.schedule_ptbs(&to_delete, resources)?;
        self.execute_calls(site_name, calls).await
    }

    /// Creates a diff between the site on chain an the resources loaded from disk
    /// The returned list contains the names of the resources to delete.
    async fn diff_site(&mut self, resources: &mut ResourceManager) -> Result<Vec<String>> {
        if self.site_id.is_none() {
            // No site - nothing to delete
            return Ok(vec![]);
        }
        let existing: HashSet<String> =
            HashSet::from_iter(self.get_existing_resource_names().await?);
        let loaded_resources = HashSet::from_iter(resources.all_names());
        tracing::info!("Existing {:?}", existing);
        tracing::info!("Loaded {:?}", loaded_resources);

        let to_delete = existing.difference(&loaded_resources);
        let common: Vec<_> = loaded_resources.intersection(&existing).collect();
        let to_create = loaded_resources.difference(&existing).collect::<Vec<_>>();
        tracing::info!("Common resources: {:?}", common);
        tracing::info!("Resources to delete: {:?}", to_delete);
        tracing::info!("Resources to create: {:?}", to_create);

        let to_update = self.check_common_for_updates(resources, &common).await?;
        tracing::info!("Resources to update: {:?}", to_update);

        // Retain resources to create or update
        resources
            .single_ptb
            .retain(|r| to_update.contains(&r.name) || to_create.contains(&&r.name));
        resources
            .multi_ptb
            .retain(|r| to_update.contains(&r.name) || to_create.contains(&&r.name));

        // Delete sites to update
        let mut delete_list: Vec<String> = to_delete.map(|s| s.to_owned()).collect();
        delete_list.extend(to_update);

        Ok(delete_list)
    }

    /// Schedule the move calls to create and delete the sites
    fn schedule_ptbs(
        &mut self,
        to_delete: &[String],
        resources: &ResourceManager,
    ) -> Result<Vec<Vec<BlocksiteCall>>> {
        let mut resources = (*resources).clone();
        let single_ptb = resources.group_by_ptb();
        let mut ptbs = vec![];

        // Finish the following single-ptb calls
        for next_batch in single_ptb {
            ptbs.push(
                next_batch
                    .iter()
                    .map(|r| r.to_ptb_calls())
                    .collect::<Result<Vec<Vec<_>>>>()?
                    .iter_mut()
                    .filter_map(|v| v.pop())
                    .flatten()
                    .collect(),
            );
        }

        for resource in &resources.multi_ptb {
            ptbs.extend(resource.to_ptb_calls()?)
        }

        // First PTB: all the deletions
        if !to_delete.is_empty() {
            let mut deletions = to_delete
                .iter()
                .map(|name| BlocksiteCall::remove_resource_if_exists(name))
                .collect::<Result<Vec<_>>>()?;

            if ptbs.is_empty() {
                ptbs.push(deletions);
            } else {
                let first = ptbs.remove(0);
                deletions.extend(first);
                ptbs.insert(0, deletions);
            }
        }

        Ok(ptbs)
    }

    /// Execute the calls
    /// If there is no site id, also create a new site in the first PTB
    async fn execute_calls(
        &mut self,
        site_name: &str,
        calls: Vec<Vec<BlocksiteCall>>,
    ) -> Result<Vec<SuiTransactionBlockResponse>> {
        let mut responses = self
            .create_site(site_name)
            .await?
            .map_or(vec![], |res| vec![res]);

        for call_ptb in calls {
            if !call_ptb.is_empty() {
                tracing::info!("Adding another PTB of calls");
                let mut ptb = self.new_ptb()?.with_call_arg(
                    &get_object_ref_from_id(
                        &self.client,
                        self.site_id.expect("Site ID should be set"),
                    )
                    .await?
                    .into(),
                )?;
                ptb.add_calls(call_ptb)?;
                responses.push(self.sign_and_send(ptb).await?);
            }
        }

        Ok(responses)
    }

    /// Create a new site and all the resources on it, and extract the object ID
    async fn create_site(
        &mut self,
        site_name: &str,
    ) -> Result<Option<SuiTransactionBlockResponse>> {
        // First ptb: create the site if necessary and add all the resources that need to be created
        if self.site_id.is_none() {
            let mut ptb = self.new_ptb()?;
            let site_arg = ptb.create_site(site_name)?;
            let mut ptb = ptb.with_arg(site_arg)?;
            if self.site_id.is_none() {
                ptb.transfer_arg(self.config.network.address(), site_arg);
            }
            let response = self.sign_and_send(ptb).await?;
            // Get the newly-created site id from the response
            self.site_id = Some(get_site_id_from_response(
                self.config.network.address(),
                &response
                    .effects
                    .clone()
                    .ok_or(anyhow!("Effects not found"))?,
            )?);
            return Ok(Some(response));
        }
        Ok(None)
    }

    fn new_ptb(&self) -> Result<BlocksitePtb> {
        BlocksitePtb::new(self.config.package, self.config.module.clone())
    }

    /// Terminates the construction of the transaction, signs it, and sends it
    /// The call builder is reset after this.
    async fn sign_and_send<T>(
        &mut self,
        ptb: BlocksitePtb<T>,
    ) -> Result<SuiTransactionBlockResponse> {
        sign_and_send_ptb(
            &self.client,
            self.config.network.keystore(),
            self.config.network.address(),
            ptb.finish(),
            get_object_ref_from_id(&self.client, self.config.gas_coin).await?,
            self.config.gas_budget,
        )
        .await
    }

    /// Check if the resources that are both on chain and on disk are the same
    /// Returns the list of resources that differ, and therefore need to be updated.
    async fn check_common_for_updates(
        &mut self,
        resources: &ResourceManager,
        common: &[&String],
    ) -> Result<Vec<String>> {
        let existing_info = get_existing_resource_ids(
            &self.client,
            &self
                .site_id
                .ok_or(anyhow!("Getting existing resources requires a site id"))?,
        )
        .await?;
        let mut to_update = vec![];
        for c in common {
            let resource_id = existing_info
                .get(c.to_owned())
                .ok_or(anyhow!("The dynamic field with name {} was not found", c))?;
            let remote_resource = self.get_remote_resource(*resource_id).await?;
            let local_resource = resources
                .get_resource_by_name(c)
                .ok_or(anyhow!("Could not find matching resource"))?;
            if remote_resource != *local_resource {
                to_update.push((*c).clone());
            }
        }
        Ok(to_update)
    }

    /// Get the resource that is hosted on chain at the given object ID
    async fn get_remote_resource(&self, object_id: ObjectID) -> Result<Resource> {
        let object = get_struct_from_object_response(
            &self
                .client
                .read_api()
                .get_object_with_options(object_id, SuiObjectDataOptions::new().with_content())
                .await?,
        )?;
        get_dynamic_field!(object, "value", SuiMoveValue::Struct).try_into()
    }

    /// Get the resources already published to the site
    async fn get_existing_resource_names(&self) -> Result<Vec<String>> {
        Ok(get_existing_resource_ids(
            &self.client,
            &self
                .site_id
                .ok_or(anyhow!("Getting existing resources requires a site id"))?,
        )
        .await?
        .into_keys()
        .collect())
    }
}

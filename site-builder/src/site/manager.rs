use std::mem;

use anyhow::{anyhow, ensure, Result};
use sui_keys::keystore::{FileBasedKeystore, Keystore};
use sui_sdk::{rpc_types::SuiTransactionBlockResponse, SuiClient};
use sui_types::{
    base_types::ObjectID,
    transaction::{Argument, CallArg},
};

use super::resource::{Resource, ResourceManager};
use crate::{
    calls::CallBuilder,
    util::{
        get_dynamic_field_names,
        get_object_ref_from_id,
        get_site_id_from_response,
        sign_and_send_ptb,
    },
    Config,
};

pub const MAX_TX_SIZE: usize = 131_072;
pub const MAX_ARG_SIZE: usize = 16_300;
pub const ARG_MARGIN: usize = 1_000; // TODO: check if there is better way
pub const TX_MARGIN: usize = 10_000; // TODO: check if there is better way
pub const MAX_OBJ_SIZE: usize = 256_000;

pub struct SiteManager {
    pub calls: CallBuilder,
    pub client: SuiClient,
    pub config: Config,
    pub keystore: Keystore,
    // The Object ID of the site under construction or update
    pub site_id: Option<ObjectID>,
}

impl SiteManager {
    pub async fn new(site_id: Option<ObjectID>, config: Config) -> Result<Self> {
        let builder = CallBuilder::new(config.package, config.module.clone());
        let client = config.network.get_sui_client().await?;
        let keystore = Keystore::File(FileBasedKeystore::new(&config.keystore)?);
        Ok(SiteManager {
            calls: builder,
            client,
            config,
            keystore,
            site_id,
        })
    }

    /// Publish a site and all relative resources
    /// Creates the site, adds all resources with size < 1 PTB, and then creates the larger resources in multiple PTBs.
    pub async fn publish_site(
        &mut self,
        site_name: &str,
        resources: &mut ResourceManager,
    ) -> Result<Vec<SuiTransactionBlockResponse>> {
        tracing::info!("Starting to publish the site");
        // Check if there are already published resources
        self.keep_missing_resources(resources).await?;
        let mut responses = vec![];
        let mut resource_iter = resources.group_by_ptb();
        if self.site_id.is_none() {
            responses.push(self.init_site(site_name, &mut resource_iter).await?);
        }
        responses.extend(self.update_site(resources, &mut resource_iter).await?);
        Ok(responses)
    }

    async fn init_site(
        &mut self,
        site_name: &str,
        resource_iter: &mut impl Iterator<Item = Vec<Resource>>,
    ) -> Result<SuiTransactionBlockResponse> {
        // Create the and add the first ptb-worth of pages
        // Only if the site manager was not configured with an object id for the site
        // (i.e., we are not continuing after crash)
        tracing::info!("No object id provided - creating new BlockSite object");
        let site_arg = self.create_site(site_name)?;
        if let Some(batch) = resource_iter.next() {
            self.create_and_add_resources(site_arg, &batch)?;
        }
        self.calls.transfer_arg(self.config.address, site_arg);
        let response = self.sign_and_send().await?;
        self.site_id = Some(get_site_id_from_response(
            self.config.address,
            &response
                .effects
                .clone()
                .ok_or(anyhow!("Effects not found"))?,
        )?);
        tracing::info!("First PTB published. New site ID: {:?}", self.site_id);
        Ok(response)
    }

    async fn update_site(
        &mut self,
        resources: &mut ResourceManager,
        resource_iter: impl Iterator<Item = Vec<Resource>>,
    ) -> Result<Vec<SuiTransactionBlockResponse>> {
        tracing::info!("Publishing subsequent batches of resources of size < PTB");
        let mut responses = vec![];
        for batch in resource_iter {
            let site_arg = self
                .arg_from_id(self.site_id.expect("By now we should have an id"))
                .await?;
            self.create_and_add_resources(site_arg, &batch)?;
            responses.push(self.sign_and_send().await?);
            tracing::info!("PTB sent");
        }
        tracing::info!("Starting to publish resources of size > PTB");
        for resource in resources.multi_ptb.iter() {
            responses.extend(self.publish_large_resource(resource).await?)
        }
        tracing::info!("Publishing completed");
        Ok(responses)
    }

    /// Single ptb
    fn create_site(&mut self, site_name: &str) -> Result<Argument> {
        let clock = self.calls.pt_builder.input(CallArg::CLOCK_IMM)?;
        let name = self.calls.pt_builder.pure(site_name)?;
        self.calls.new_site(name, clock)
    }

    /// Single ptb
    pub fn create_and_add_resources(
        &mut self,
        site_arg: Argument,
        resources: &[Resource],
    ) -> Result<()> {
        for res in resources {
            self.create_and_add_resource(site_arg, res)?
        }
        Ok(())
    }

    /// Single ptb
    fn create_and_add_resource(&mut self, site_arg: Argument, resource: &Resource) -> Result<()> {
        let page_arg = self.create_resource_in_chunks(resource, None, false)?;
        self.calls.add_resource(site_arg, page_arg)?;
        tracing::info!("Added resource {} to PTB", resource.name);
        Ok(())
    }

    /// Single ptb
    fn create_resource_in_chunks(
        &mut self,
        resource: &Resource,
        chunks: Option<&Vec<&[u8]>>,
        temporary: bool,
    ) -> Result<Argument> {
        // Create separate chunks of at most MAX_ARG_LEN
        let chunks = match chunks {
            Some(c) => c.to_vec(),
            None => resource.content.chunks(MAX_ARG_SIZE - ARG_MARGIN).collect(),
        };
        let first_chunk_arg = self.calls.pt_builder.pure(chunks[0])?;
        let (clock, name, content_type, content_encoding) =
            self.resource_args(resource, temporary)?;
        let resource_arg = self.calls.new_resource(
            name,
            content_type,
            content_encoding,
            first_chunk_arg,
            clock,
        )?;
        self.resource_add_chunks(&chunks[1..], resource_arg, clock)?;
        Ok(resource_arg)
    }

    fn resource_add_chunks(
        &mut self,
        chunks: &[&[u8]],
        resource: Argument,
        clock: Argument,
    ) -> Result<()> {
        for chunk in chunks {
            let chunk_arg = self.calls.pt_builder.pure(chunk)?;
            self.calls.add_piece(resource, chunk_arg, clock)?;
        }
        Ok(())
    }

    fn resource_args(
        &mut self,
        resource: &Resource,
        temporary: bool,
    ) -> Result<(Argument, Argument, Argument, Argument)> {
        let clock = self.calls.pt_builder.input(CallArg::CLOCK_IMM)?;
        let name = self.calls.pt_builder.pure(if temporary {
            resource.tmp_path()
        } else {
            resource.name.clone()
        })?;
        let content_type = self
            .calls
            .pt_builder
            .pure(resource.content_type.to_string())?;
        let content_encoding = self
            .calls
            .pt_builder
            .pure(resource.content_encoding.to_string())?;
        Ok((clock, name, content_type, content_encoding))
    }

    /// Terminates the construction of the transaction, signs it, and sends it
    /// The call builder is reset after this.
    async fn sign_and_send(&mut self) -> Result<SuiTransactionBlockResponse> {
        let builder = mem::replace(
            &mut self.calls,
            CallBuilder::new(self.config.package, self.config.module.clone()),
        );
        sign_and_send_ptb(
            &self.client,
            &self.keystore,
            self.config.address,
            builder.finish(),
            get_object_ref_from_id(&self.client, self.config.gas_coin).await?,
            self.config.gas_budget,
        )
        .await
    }

    /// Multi-ptb resource publishing
    /// Create the page under a /tmp/ resource, fill it, and move it.
    pub async fn publish_large_resource(
        &mut self,
        resource: &Resource,
    ) -> Result<Vec<SuiTransactionBlockResponse>> {
        ensure!(
            resource.size_in_ptb() < MAX_OBJ_SIZE,
            "Resource {} too large with size {}",
            resource.name,
            resource.size_in_ptb(),
        );
        let ptb_chunks: Vec<Vec<&[u8]>> = resource
            .content
            .chunks(MAX_TX_SIZE - TX_MARGIN)
            .map(|c| c.chunks(MAX_ARG_SIZE - ARG_MARGIN).collect())
            .collect();

        // First ptb removes any remaining temporary resources & creates the resource
        let site_arg = self
            .arg_from_id(
                self.site_id
                    .ok_or(anyhow!("There should be an object id provided"))?,
            )
            .await?;

        let old_resource_arg = self.calls.pt_builder.pure(resource.tmp_path())?;
        self.calls
            .remove_resource_if_exists(site_arg, old_resource_arg)?;
        let resource_arg = self.create_resource_in_chunks(resource, Some(&ptb_chunks[0]), true)?;
        self.calls.add_resource(site_arg, resource_arg)?;
        let response = self.sign_and_send().await?;
        let mut responses = vec![response];
        tracing::info!("Created temporary resource {}", resource.tmp_path());

        // All follwing ptbs - This must be executed at least once
        for (idx, ptb) in ptb_chunks[1..].iter().enumerate() {
            self.add_chunks_to_existing(ptb, resource).await?;
            tracing::info!("Added PTB to temporary resource");
            if idx == ptb_chunks.len() - 2 {
                // If we are at the last iteration, move to the correct name
                let old_name = self.calls.pt_builder.pure(resource.tmp_path())?;
                let new_name = self.calls.pt_builder.pure(resource.name.clone())?;
                self.calls.move_resource(site_arg, old_name, new_name)?;
                tracing::info!(
                    "Moved temporary resource {} to destination {}",
                    resource.tmp_path(),
                    resource.name
                );
            }
            responses.push(self.sign_and_send().await?);
        }
        Ok(responses)
    }

    // Add chunks to an existing resource under the tmp name
    async fn add_chunks_to_existing(
        &mut self,
        chunks: &[&[u8]],
        resource: &Resource,
    ) -> Result<()> {
        let site_arg = self
            .arg_from_id(self.site_id.expect("There should be a site ID"))
            .await?;
        let name_arg = self.calls.pt_builder.pure(resource.tmp_path())?;
        let clock = self.calls.pt_builder.input(CallArg::CLOCK_IMM)?;
        self.resource_add_chunks_to_existing(chunks, site_arg, name_arg, clock)?;
        Ok(())
    }

    fn resource_add_chunks_to_existing(
        &mut self,
        chunks: &[&[u8]],
        site: Argument,
        name: Argument,
        clock: Argument,
    ) -> Result<()> {
        for chunk in chunks {
            let chunk_arg = self.calls.pt_builder.pure(chunk)?;
            self.calls
                .add_piece_to_existing(site, name, chunk_arg, clock)?;
        }
        Ok(())
    }

    async fn arg_from_id(&mut self, object: ObjectID) -> Result<Argument> {
        self.calls
            .pt_builder
            .input(get_object_ref_from_id(&self.client, object).await?.into())
    }

    /// Modifies the [ResourceManager] in place to only keep the resources that are missing from the object
    /// If there is no [site_id] specified, keep all the resources.
    async fn keep_missing_resources(&mut self, resources: &mut ResourceManager) -> Result<()> {
        if self.site_id.is_none() {
            // If the site has not been specified, we need to publish everything
            return Ok(());
        }
        tracing::info!("Object id was provided. Verifying if there are missing resources.");
        let existing = get_dynamic_field_names(&self.client, self.site_id.unwrap()).await?;
        resources
            .single_ptb
            .retain(|res| !existing.contains(&res.name));
        resources
            .multi_ptb
            .retain(|res| !existing.contains(&res.name));
        tracing::info!("Missing resources to be added:\n{}", &resources);
        Ok(())
    }
}

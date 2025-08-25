// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{collections::BTreeMap, path::PathBuf};

use anyhow::anyhow;

use crate::{
    args::{default, EpochArg, PublishOptions, WalrusStoreOptions},
    config::{Config, Walrus},
    display,
    publish::load_ws_resources,
    site::{builder::SitePtb, config::WSResources, resource::Resource},
    site_new::resource::{local_resource::LocalResource, manager::ResourceManager},
    types::SuiResource,
    walrus::{
        command::{QuiltBlobInput, StoreQuiltInput},
        StoreQuiltArguments,
        WalrusOp,
    },
};

/// Separates the argument parsing from actually building the site.
#[derive(Debug)]
pub struct SitePublisherBuilder {
    pub context: Option<String>,
    pub site_name: Option<String>,
    pub config: Config,
    pub publish_options: PublishOptions,
}

impl SitePublisherBuilder {
    // pub fn with_context(mut self, context: Option<String>) -> Self {
    //     self.context = context;
    //     self
    // }
    //
    // pub fn with_site_name(mut self, site_name: String) -> Self {
    //     self.site_name = Some(site_name);
    //     self
    // }
    //
    // pub fn with_publish_options(mut self, publish_options: PublishOptions) -> Self {
    //     self.publish_options = publish_options;
    //     self
    // }

    pub async fn build(self) -> anyhow::Result<SitePublisher> {
        let Self {
            context,
            site_name,
            config,
            publish_options,
        } = self;
        let PublishOptions {
            directory,
            list_directory: _,      // TODO(nikos) handle list-directory
            max_parallel_stores: _, // TODO(nikos) will proly need this later
            walrus_options:
                WalrusStoreOptions {
                    ws_resources,
                    epoch_arg,
                    permanent,
                    dry_run,
                },
            ..
        } = publish_options;
        let (ws_resources, ws_resources_path) =
            load_ws_resources(ws_resources.as_deref(), directory.as_path())?;
        let WSResources {
            headers,
            routes: _,   // TODO(nikos) will proly need this later
            metadata: _, // TODO(nikos) will proly need this later
            site_name: ws_site_name,
            object_id: _, // TODO(nikos) will proly need this later
            ignore,
        } = ws_resources.unwrap_or_default();
        let site_name = site_name
            .or(ws_site_name)
            .unwrap_or(default::DEFAULT_SITE_NAME.to_string());

        let resource_manager = ResourceManager::new(
            headers.unwrap_or_default(),
            ignore.unwrap_or_default(),
            ws_resources_path,
        );

        let walrus = Walrus::new(
            config.walrus_binary(),
            config.gas_budget(),
            config.general.rpc_url.clone(),
            config.general.walrus_config.clone(),
            config.general.walrus_context.clone(),
            config.general.wallet.clone(),
        );
        Ok(SitePublisher {
            context,
            site_name,
            resource_manager,
            directory,
            epoch_arg,
            permanent,
            dry_run,
            walrus,
        })
    }
}

// TODO(nikos): Handle list-directory. To me it makes sense to be a separate command.
// Also I think it will be deprecated after File-manager in walrus is implemented.
pub struct SitePublisher {
    pub context: Option<String>,
    pub site_name: String,
    // TODO(nikos): We probably need to keep the path of the ws-resources in order to not upload.
    pub resource_manager: ResourceManager,
    // TODO(nikos): Does it make sense to include directory inside the new `ResourceManager` above?
    pub directory: PathBuf,
    pub epoch_arg: EpochArg,
    pub permanent: bool,
    pub dry_run: bool,
    pub walrus: Walrus,
}

impl SitePublisher {
    pub async fn run(self) -> anyhow::Result<(Vec<WalrusOp>, SitePtb)> {
        let Self {
            context: _,   // TODO(nikos) will proly need this later
            site_name: _, // TODO(nikos) will proly need this later
            mut resource_manager,
            directory,
            epoch_arg,
            permanent,
            dry_run,
            mut walrus,
        } = self;

        display::action(format!(
            "Parsing the directory {}",
            directory.to_string_lossy()
        ));
        let resources = resource_manager.read_dir(directory.as_path())?;
        display::done();
        tracing::debug!(?resources, "resources loaded from directory");

        tracing::debug!("creating site");

        let local_resources_chunks: Vec<Vec<_>> = resources
            .chunks(Walrus::MAX_FILES_PER_QUILT)
            .map(|r| r.to_vec())
            .collect();
        let walrus_ops = local_resources_chunks
            .into_iter()
            .map(|resources| {
                let args = StoreQuiltArguments {
                    store_quilt_input: StoreQuiltInput::Blobs(
                        resources
                            .iter()
                            .map(|resource| {
                                QuiltBlobInput {
                                    path: resource.full_path.clone(),
                                    // TODO(nikos): error on not-supported characters
                                    identifier: Some(resource.info.path.clone()),
                                    tags: BTreeMap::new(),
                                }
                            })
                            .collect(),
                    ),
                    epoch_arg: epoch_arg.clone(),
                    deletable: !permanent,
                };
                let op = if dry_run {
                    WalrusOp::DryRunStoreQuilt(args)
                } else {
                    WalrusOp::StoreQuilt(args)
                };
                (resources, op)
            })
            .collect::<Vec<_>>();

        let walrus_resps = {
            let mut walrus_resps = vec![];
            for op in walrus_ops {
                walrus_resps.push((op.0, walrus.run(op.1).await?));
            }
            walrus_resps
        };

        let flattened: Vec<Resource> = walrus_resps
            .into_iter()
            .flat_map(|(chunk, resp)| {
                let mut patches = resp.patch_ids()?;
                let blob_id = *resp.blob_id();
                let resources = chunk
                    .into_iter()
                    .map(
                        |LocalResource {
                             info,
                             full_path,
                             unencoded_size,
                         }| {
                            let path = info.path.as_str();
                            let patch_idx = patches
                                .iter()
                                .position(|p| path == p.0)
                                .ok_or(anyhow!("Did not find {path} in walrus response"))?;
                            let patch = patches.swap_remove(patch_idx);
                            let sui_resource = SuiResource::from((info, blob_id, patch.1));
                            Ok(Resource {
                                info: sui_resource,
                                full_path,
                                unencoded_size,
                            })
                        },
                    )
                    .collect::<anyhow::Result<Vec<Resource>>>()?;
                Ok::<Vec<Resource>, anyhow::Error>(resources)
            })
            .flatten()
            .collect();

        // let site_updates = local_site_data.diff(&existing_site);
        //
        // let walrus_candidate_set = if self.blob_options.is_check_extend() {
        //     // We need to check the status of all blobs: Return the full list of existing and added
        //     // blobs as possible updates.
        //     existing_site.replace_all(local_site_data)
        // } else {
        //     // We only need to upload the new blobs.
        //     site_updates.clone()
        // };
        // // IMPORTANT: Perform the store operations on Walrus first, to ensure zero "downtime".
        // self.select_and_store_to_walrus(&walrus_candidate_set)
        //     .await?;
        //
        // // Check if there are any updates to the site on-chain.
        // let result = if site_updates.has_updates() {
        //     display::action("Applying the Walrus Site object updates on Sui");
        //     let result = self.execute_sui_updates(&site_updates).await?;
        //     display::done();
        //     result
        // } else {
        //     SuiTransactionBlockResponse::default()
        // };

        todo!();
    }
}

// Gets the configuration from the provided file, or looks in the default directory.
/*
    async fn run_single_edit(
        &self,
    ) -> Result<(SuiAddress, SuiTransactionBlockResponse, SiteDataDiffSummary)> {
        if self.edit_options.publish_options.list_directory {
            display::action(format!("Preprocessing: {}", self.directory().display()));
            Preprocessor::preprocess(self.directory())?;
            display::done();
        }

        // Note: `load_ws_resources` again. We already loaded them when parsing the name.
        let (ws_resources, ws_resources_path) = load_ws_resources(
            self.edit_options
                .publish_options
                .walrus_options
                .ws_resources
                .as_deref(),
            self.directory(),
        )?;
        if let Some(path) = ws_resources_path.as_ref() {
            println!(
                "Using the Walrus sites resources file: {}",
                path.to_string_lossy()
            );
        }

        let mut resource_manager = ResourceManager::new(
            self.config.walrus_client(),
            ws_resources.clone(),
            ws_resources_path.clone(),
            self.edit_options.publish_options.max_concurrent,
        )
        .await?;
        display::action(format!(
            "Parsing the directory {} and locally computing blob IDs",
            self.directory().to_string_lossy()
        ));
        let local_site_data = resource_manager.read_dir(self.directory()).await?;
        display::done();
        tracing::debug!(?local_site_data, "resources loaded from directory");

        let site_metadata = match ws_resources.clone() {
            Some(value) => value.metadata,
            None => None,
        };

        let site_name = ws_resources.as_ref().and_then(|r| r.site_name.clone());

        let mut site_manager = SiteManager::new(
            self.config.clone(),
            self.edit_options.site_id,
            self.edit_options.blob_options.clone(),
            self.edit_options.publish_options.walrus_options.clone(),
            site_metadata,
            self.edit_options.site_name.clone().or(site_name),
            self.edit_options.publish_options.max_parallel_stores,
        )
        .await?;

        let (response, summary) = site_manager.update_site(&local_site_data).await?;

        let path_for_saving =
            ws_resources_path.unwrap_or_else(|| self.directory().join(DEFAULT_WS_RESOURCES_FILE));

        persist_site_identifier(
            &self.edit_options.site_id,
            &site_manager,
            &response,
            ws_resources,
            &path_for_saving,
        )?;

        Ok((site_manager.active_address()?, response, summary))
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

        let walrus_candidate_set = if self.blob_options.is_check_extend() {
            // We need to check the status of all blobs: Return the full list of existing and added
            // blobs as possible updates.
            existing_site.replace_all(local_site_data)
        } else {
            // We only need to upload the new blobs.
            site_updates.clone()
        };
        // IMPORTANT: Perform the store operations on Walrus first, to ensure zero "downtime".
        self.select_and_store_to_walrus(&walrus_candidate_set)
            .await?;

        // Check if there are any updates to the site on-chain.
        let result = if site_updates.has_updates() {
            display::action("Applying the Walrus Site object updates on Sui");
            let result = self.execute_sui_updates(&site_updates).await?;
            display::done();
            result
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

*/

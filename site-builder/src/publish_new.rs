// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    path::PathBuf,
    str::FromStr,
};

use anyhow::anyhow;
use sui_sdk::{
    rpc_types::{SuiTransactionBlockEffectsAPI, SuiTransactionBlockResponse},
    wallet_context::WalletContext,
};
use sui_types::{base_types::ObjectID, transaction::CallArg, Identifier};

use crate::{
    args::{default, EpochArg, PublishOptions, WalrusStoreOptions},
    backoff::ExponentialBackoffConfig,
    config::{Config, Walrus},
    display,
    publish::load_ws_resources,
    retry_client::RetriableSuiClient,
    site::{
        builder::SitePtb,
        config::WSResources,
        resource::{Resource, ResourceOp},
        SiteDataDiff,
        SITE_MODULE,
    },
    site_new::resource::{local_resource::LocalResource, manager::ResourceManager},
    types::{Metadata, MetadataOp, RouteOps, Routes, SiteNameOp, SuiResource},
    util::sign_and_send_ptb,
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
            routes,
            metadata,
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
            routes,
            metadata,
            wallet: config.load_wallet()?,
            gas_budget: config.gas_budget(),
            package: config.package,
        })
    }
}

// TODO(nikos): Handle list-directory. To me it makes sense to be a separate command.
// Also I think it will be deprecated after File-manager in walrus is implemented.
pub struct SitePublisher {
    pub context: Option<String>,
    pub site_name: String,
    // TODO(nikos): We need to keep the path of the ws-resources in order to not upload.
    pub resource_manager: ResourceManager,
    // TODO(nikos): Does it make sense to include directory inside the new `ResourceManager` above?
    pub directory: PathBuf,
    pub epoch_arg: EpochArg,
    pub permanent: bool,
    pub dry_run: bool,
    pub walrus: Walrus,
    pub routes: Option<Routes>,
    pub metadata: Option<Metadata>,
    pub wallet: WalletContext,
    pub gas_budget: u64,
    pub package: ObjectID,
}

impl SitePublisher {
    pub async fn run(self) -> anyhow::Result<Vec<SuiTransactionBlockResponse>> {
        let Self {
            context: _, // TODO(nikos) will proly need this later
            site_name,  // TODO(nikos) will proly need this later
            mut resource_manager,
            directory,
            epoch_arg,
            permanent,
            dry_run,
            mut walrus,
            routes,
            metadata,
            mut wallet,
            gas_budget,
            package,
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
        println!("local_resources_chunks: {:#?}", local_resources_chunks);

        let mut walrus_ops = vec![];
        let mut path_to_identifier = HashMap::new();
        for resources_chunk in local_resources_chunks {
            let mut blobs = vec![];
            for resource in &resources_chunk {
                // TODO(nikos): handle not-supported characters
                let resource_path = resource.info.path.clone();
                let mut identifier = if resource_path.chars().next() == Some('/') {
                    &resource_path[1..]
                } else {
                    resource_path.as_str()
                }
                .to_string();
                identifier = identifier.replace("/", "__");
                path_to_identifier.insert(resource_path.clone(), identifier.clone());
                blobs.push(QuiltBlobInput {
                    path: resource.full_path.clone(),
                    identifier: Some(identifier),
                    // TODO(nikos) determine path
                    tags: BTreeMap::from([("path".to_string(), resource_path)]),
                });
            }
            let args = StoreQuiltArguments {
                store_quilt_input: StoreQuiltInput::Blobs(blobs),
                epoch_arg: epoch_arg.clone(),
                deletable: !permanent,
            };
            walrus_ops.push((
                resources_chunk,
                if dry_run {
                    WalrusOp::DryRunStoreQuilt(args)
                } else {
                    WalrusOp::StoreQuilt(args)
                },
            ));
        }
        println!("walrus_ops {:#?}", walrus_ops);

        let walrus_resps = {
            let mut walrus_resps = vec![];
            for op in walrus_ops {
                walrus_resps.push((op.0, walrus.run(op.1).await?));
            }
            walrus_resps
        };
        println!("walrus_resps: {:#?}", walrus_resps);

        let resources: Vec<Resource> = walrus_resps
            .into_iter()
            .map(|(chunk, resp)| {
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
                            let identifier = path_to_identifier
                                .get(path)
                                .ok_or(anyhow!("Expected {path} in the map"))?;
                            let patch_idx = patches
                                .iter()
                                .position(|p| identifier == p.0)
                                .ok_or(anyhow!("Did not find {identifier} in walrus response"))?;
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
            .collect::<anyhow::Result<Vec<Vec<Resource>>>>()?
            .into_iter()
            .flatten()
            .collect();
        println!("resources: {:#?}", resources);

        let route_ops = match routes {
            Some(r) => RouteOps::Replace(r),
            None => RouteOps::Unchanged,
        };
        let metadata_op = match metadata {
            Some(_) => MetadataOp::Update,
            None => MetadataOp::Noop,
        };

        let resource_ops = resources.iter().map(|r| ResourceOp::Created(r)).collect();

        let updates = SiteDataDiff {
            route_ops,
            metadata_op,
            resource_ops,
            site_name_op: SiteNameOp::Update,
        };

        SitePublisher::execute_sui_updates(
            &mut wallet,
            package,
            site_name.as_str(),
            metadata,
            gas_budget,
            &updates,
        )
        .await
    }

    async fn execute_sui_updates(
        wallet: &mut WalletContext,
        package: ObjectID,
        site_name: &str,
        metadata: Option<Metadata>,
        gas_budget: u64,
        updates: &SiteDataDiff<'_>,
    ) -> anyhow::Result<Vec<SuiTransactionBlockResponse>> {
        let active_address = wallet.active_address()?;
        tracing::debug!(
            address=?active_address,
            ?updates,
            "starting to update site resources on chain",
        );
        let ptb = SitePtb::new(
            package,
            Identifier::from_str(SITE_MODULE).expect("the str provided is valid"),
        )?;

        let mut ptb = ptb.with_create_site(site_name, metadata)?;
        ptb.update_name(site_name)?;

        // Publish the first MAX_RESOURCES_PER_PTB resources, or all resources if there are fewer
        // than that.
        const MAX_RESOURCES_PER_PTB: usize = 200;
        let mut end = MAX_RESOURCES_PER_PTB.min(updates.resource_ops.len());
        tracing::debug!(
            total_ops = updates.resource_ops.len(),
            end,
            "preparing and committing the first PTB"
        );

        println!("resource_ops: {:#?}", &updates.resource_ops[..end]);
        ptb.add_resource_operations(&updates.resource_ops[..end])?;
        ptb.add_route_operations(&updates.route_ops)?;

        ptb.transfer_site(active_address);

        let backoff_config = ExponentialBackoffConfig::default();
        let sui_client = RetriableSuiClient::new_from_wallet(wallet, backoff_config).await?;
        let gas_coin = wallet
            .gas_for_owner_budget(active_address, gas_budget, BTreeSet::new())
            .await?
            .1
            .object_ref();
        let resp = sign_and_send_ptb(
            active_address,
            wallet,
            &sui_client,
            ptb.finish(),
            gas_coin,
            gas_budget,
        )
        .await?;
        let effects = resp
            .effects
            .as_ref()
            .ok_or(anyhow!("the result did not have effects"))?;

        tracing::debug!(
            ?effects,
            "getting the object ID of the created Walrus site."
        );

        println!("effects: {}", serde_json::to_string_pretty(effects)?);

        let site_object_id = effects
            .created()
            .iter()
            .find(|c| {
                c.owner
                    .get_owner_address()
                    .map(|owner_address| owner_address == active_address)
                    .unwrap_or(false)
            })
            .expect("could not find the object ID for the created Walrus site.")
            .reference
            .object_id;

        let mut result = vec![resp];

        // Keep iterating to load resources.
        while end < updates.resource_ops.len() {
            let start = end;
            end = (end + MAX_RESOURCES_PER_PTB).min(updates.resource_ops.len());
            tracing::debug!(%start, %end, "preparing and committing the next PTB");

            let ptb = SitePtb::new(
                package,
                Identifier::from_str(SITE_MODULE).expect("the str provided is valid"),
            )?;
            let call_arg: CallArg = wallet.get_object_ref(site_object_id).await?.into();
            let mut ptb = ptb.with_call_arg(&call_arg)?;
            ptb.add_resource_operations(&updates.resource_ops[start..end])?;

            // TODO: Optimize this
            let gas_coin = wallet
                .gas_for_owner_budget(active_address, gas_budget, BTreeSet::new())
                .await?
                .1
                .object_ref();

            result.push(
                sign_and_send_ptb(
                    active_address,
                    wallet,
                    &sui_client,
                    ptb.finish(),
                    gas_coin,
                    gas_budget,
                )
                .await?,
            );
        }

        Ok(result)
    }
}

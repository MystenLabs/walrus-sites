// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::{BTreeSet, HashSet},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};
use sui_sdk::rpc_types::{
    SuiExecutionStatus,
    SuiTransactionBlockEffects,
    SuiTransactionBlockResponse,
};
use sui_types::{
    base_types::{ObjectID, SuiAddress},
    Identifier,
};

use crate::{
    args::PublishOptions,
    backoff::ExponentialBackoffConfig,
    config::Config,
    display,
    preprocessor::Preprocessor,
    retry_client::RetriableSuiClient,
    site::{
        builder::{SitePtb, PTB_MAX_MOVE_CALLS},
        config::WSResources,
        manager::SiteManager,
        resource::ResourceManager,
        RemoteSiteFactory,
        SITE_MODULE,
    },
    summary::{SiteDataDiffSummary, Summarizable},
    types::ObjectCache,
    util::{
        get_site_id_from_response,
        id_to_base36,
        path_or_defaults_if_exist,
        persist_site_id_and_name,
        sign_and_send_ptb,
    },
};

const DEFAULT_WS_RESOURCES_FILE: &str = "ws-resources.json";

pub(crate) struct EditOptions {
    pub publish_options: PublishOptions,
    pub site_id: Option<ObjectID>,
    pub site_name: Option<String>,
}

pub(crate) struct SiteEditor<E = ()> {
    context: Option<String>,
    config: Config,
    edit_options: E,
}

impl SiteEditor {
    pub fn new(context: Option<String>, config: Config) -> Self {
        SiteEditor {
            context,
            config,
            edit_options: (),
        }
    }

    pub fn with_edit_options(
        self,
        publish_options: PublishOptions,
        site_id: Option<ObjectID>,
        site_name: Option<String>,
    ) -> SiteEditor<EditOptions> {
        SiteEditor {
            context: self.context,
            config: self.config,
            edit_options: EditOptions {
                publish_options,
                site_id,
                site_name,
            },
        }
    }

    pub async fn destroy(&self, site_id: ObjectID) -> Result<()> {
        // Delete blobs on Walrus.
        let wallet_walrus = self.config.load_wallet()?;
        let retriable_client = RetriableSuiClient::new_from_wallet(
            &wallet_walrus,
            ExponentialBackoffConfig::default(),
        )
        .await?;

        let site = RemoteSiteFactory::new(
            // TODO(giac): make the backoff configurable.
            &retriable_client,
            self.config.package,
        )
        .await?
        .get_from_chain(site_id)
        .await?;

        let all_blobs: Vec<_> = site
            .resources()
            .into_iter()
            .map(|resource| resource.info.blob_id)
            // Collect first to a hash-set to keep unique blob-ids.
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        tracing::debug!(?all_blobs, "retrieved the site for deletion");

        // Add warning if no deletable blobs found.
        if all_blobs.is_empty() {
            println!(
                "Warning: No deletable resources found. This may be because the site was created with permanent=true"
            );
        } else {
            let mut site_manager =
                SiteManager::new(self.config.clone(), Some(site_id), None, None).await?;

            site_manager.delete_from_walrus(&all_blobs).await?;
        }

        // Delete objects on SUI blockchain
        let mut wallet = self.config.load_wallet()?;
        let active_address = wallet.active_address()?;
        
        let site = RemoteSiteFactory::new(&retriable_client, self.config.package)
            .await?
            .get_from_chain(site_id)
            .await?;
        for resource in site.resources().batch(998) {
            let ptb = SitePtb::<_, PTB_MAX_MOVE_CALLS>::new(
                self.config.package,
                Identifier::new(SITE_MODULE)?,
            );
            let mut ptb = ptb.with_call_arg(&wallet.get_object_ref(site_id).await?.into())?;
    
            ptb.destroy(&resource)?;
            let gas_coin = wallet
                .gas_for_owner_budget(active_address, self.config.gas_budget(), BTreeSet::new())
                .await?
                .1
                .object_ref();
    
            sign_and_send_ptb(
                active_address,
                &wallet,
                &retriable_client,
                ptb.finish(),
                gas_coin,
                self.config.gas_budget(),
                &mut ObjectCache::new(),
            )
            .await?;
        }
        
        let last_ptb = SitePtb::<_, PTB_MAX_MOVE_CALLS>::new(
            self.config.package,
            Identifier::new(SITE_MODULE)?,
        ); 
        let mut postprocess_ptb = last_ptb.with_call_arg(&wallet.get_object_ref(site_id).await?.into())?;
        postprocess_ptb.burn()?;
        postprocess_ptb.remove_routes()?;
        let gas_coin = wallet
            .gas_for_owner_budget(active_address, self.config.gas_budget(), BTreeSet::new())
            .await?
            .1
            .object_ref();
        sign_and_send_ptb(
            active_address,
            &wallet,
            &retriable_client,
            postprocess_ptb.finish(),
            gas_coin,
            self.config.gas_budget(),
            &mut ObjectCache::new(),
        )
        .await?;
        
        Ok(())
    }
}

impl SiteEditor<EditOptions> {
    /// The directory containing the site sources.
    pub fn directory(&self) -> &Path {
        &self.edit_options.publish_options.directory
    }

    pub async fn run_quilts(&self) -> Result<()> {
        let (active_address, response, summary) = self.run_single_edit_quilts().await?;
        print_summary(
            &self.config,
            &active_address,
            &self.edit_options.site_id,
            &response,
            &summary,
        )?;
        Ok(())
    }

    async fn run_single_edit_quilts(
        &self,
    ) -> Result<(SuiAddress, SuiTransactionBlockResponse, SiteDataDiffSummary)> {
        let (mut resource_manager, mut site_manager) = self.create_managers().await?;
        if self.is_list_directory() {
            self.preprocess_directory(&resource_manager)?;
        }

        display::action(format!(
            "Parsing the directory {}, computing Quilt IDs, and storing Quilts",
            self.directory().to_string_lossy()
        ));
        let dry_run = self.edit_options.publish_options.walrus_options.dry_run;
        let local_site_data = resource_manager
            .read_dir_and_store_quilts(
                self.directory(),
                self.edit_options
                    .publish_options
                    .walrus_options
                    .epoch_arg
                    .clone(),
                dry_run,
                self.edit_options
                    .publish_options
                    .walrus_options
                    .max_quilt_size,
            )
            .await?;
        display::done();
        tracing::debug!(
            ?local_site_data,
            "resources loaded and stored from directory"
        );

        let (response, summary) = site_manager.update_site(&local_site_data).await?;

        self.persist_site_identifier(resource_manager, &site_manager, &response)?;

        Ok((site_manager.active_address()?, response, summary))
    }

    async fn create_managers(&self) -> Result<(ResourceManager, SiteManager)> {
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

        let resource_manager =
            ResourceManager::new(self.config.walrus_client(), ws_resources, ws_resources_path)
                .await?;

        let site_metadata = match resource_manager.ws_resources.clone() {
            Some(value) => value.metadata,
            None => None,
        };

        let site_name = resource_manager
            .ws_resources
            .as_ref()
            .and_then(|r| r.site_name.clone());

        let site_manager = SiteManager::new(
            self.config.clone(),
            self.edit_options.site_id,
            site_metadata,
            self.edit_options.site_name.clone().or(site_name),
        )
        .await?;

        Ok((resource_manager, site_manager))
    }

    fn persist_site_identifier(
        &self,
        resource_manager: ResourceManager,
        site_manager: &SiteManager,
        response: &SuiTransactionBlockResponse,
    ) -> Result<()> {
        let path_for_saving = resource_manager
            .ws_resources_path
            .unwrap_or_else(|| self.directory().join(DEFAULT_WS_RESOURCES_FILE));

        persist_site_identifier(
            &self.edit_options.site_id,
            site_manager,
            response,
            resource_manager.ws_resources,
            &path_for_saving,
        )
    }

    /// Returns whether the list_directory option is enabled.
    fn is_list_directory(&self) -> bool {
        self.edit_options.publish_options.list_directory
    }

    /// Runs the preprocessing step on the directory.
    fn preprocess_directory(&self, resource_manager: &ResourceManager) -> Result<()> {
        display::action(format!("Preprocessing: {}", self.directory().display()));
        let _ = Preprocessor::preprocess(
            self.directory(),
            &resource_manager
                .ws_resources
                .as_ref()
                .and_then(|ws| ws.ignore.clone()),
        );
        display::action(format!(
            "Successfully preprocessed the {} directory!",
            self.directory().display()
        ));
        display::done();
        Ok(())
    }
}

fn print_summary(
    config: &Config,
    address: &SuiAddress,
    site_id: &Option<ObjectID>,
    response: &SuiTransactionBlockResponse,
    summary: &impl Summarizable,
) -> Result<()> {
    if let Some(SuiTransactionBlockEffects::V1(eff)) = response.effects.as_ref() {
        if let SuiExecutionStatus::Failure { error } = &eff.status {
            return Err(anyhow!(
                "error while processing the Sui transaction: {error}"
            ));
        }
    }

    display::header("Execution completed");
    println!("{}\n", summary.to_summary());
    let object_id = match site_id {
        Some(id) => {
            println!("Site object ID: {id}");
            *id
        }
        None => {
            let id = get_site_id_from_response(
                *address,
                response
                    .effects
                    .as_ref()
                    .ok_or(anyhow::anyhow!("response did not contain effects"))?,
            )?;
            println!("Created new site! \nNew site object ID: {id}");
            id
        }
    };
    let is_mainnet = matches!(config.general.walrus_context.as_deref(), Some("mainnet"));

    if is_mainnet {
        println!(
            r#"To browse your mainnet site, you have the following options:
    1. Run a mainnet portal locally, and browse the site through it: e.g. http://{base36_id}.localhost:3000
       (more info: https://docs.wal.app/walrus-sites/portal.html#running-the-portal-locally)
    2. Use a third-party portal (e.g. wal.app), which will require a SuiNS name.
       First, buy a SuiNS name at suins.io (e.g. example-domain), then point it to the site object ID.
       Finally, browse it with: https://example-domain.{portal}"#,
            base36_id = id_to_base36(&object_id)?,
            portal = config.portal
        );
    } else {
        println!(
            r#"⚠️ wal.app only supports sites deployed on mainnet.
     To browse your testnet site, you need to self-host a portal:
     1. For local development: http://{base36_id}.localhost:3000
     2. For public sharing: http://{base36_id}.yourdomain.com:3000

     📖 Setup instructions: https://docs.wal.app/walrus-sites/portal.html#running-the-portal-locally

     💡 Tip: You may also bring your own domain (https://docs.wal.app/walrus-sites/bring-your-own-domain.html)
            or find third-party hosted testnet portals."#,
            base36_id = id_to_base36(&object_id)?
        );
    }
    Ok(())
}

/// Persists the site identifier (ID and optional name) into the `ws-resources.json` file.
///
/// This function handles the following:
/// - For an existing site, it logs the site ID and persists it.
/// - For a newly published site, it extracts the new site ID from the transaction effects
///   and persists both the ID and the user-provided name.
///
/// # Arguments
///
/// * `site_id` - A reference to the `SiteIdentifier` which provides the object id if existing site,
///   or the site_name if a new site
/// * `site_manager` - A reference to the `SiteManager` which provides access to the active address.
/// * `response` - The transaction response containing the effects used to extract the new site ID.
/// * `ws_resources` - The current workspace resources to be updated and saved.
/// * `path` - The path to which the `ws-resources.json` should be written.
///
/// # Errors
///
/// Returns an error if the active address or transaction effects are missing,
/// or if the persistence operation fails.
fn persist_site_identifier(
    site_id: &Option<ObjectID>,
    site_manager: &SiteManager,
    response: &SuiTransactionBlockResponse,
    ws_resources: Option<WSResources>, // TODO: theoretically we should only need a ref.
    path: &Path,
) -> Result<()> {
    match site_id {
        Some(object_id) => {
            tracing::info!(
                "Operation was on an existing site (ID: {}). This ID will be persisted in ws-resources.json.",
                object_id
            );
            persist_site_id_and_name(*object_id, None, ws_resources, path)?;
        }
        None => {
            let active_address = site_manager.active_address()?;

            let tx_effects = response
                .effects
                .as_ref()
                .ok_or_else(|| anyhow!("Transaction effects not found"))?;

            let new_site_object_id = get_site_id_from_response(active_address, tx_effects)?;

            tracing::info!(
                "New site published. New ObjectID ({}) will be persisted in ws-resources.json.",
                new_site_object_id
            );
            let name = ws_resources
                .as_ref()
                .and_then(|r| site_manager.site_name.clone().or(r.site_name.clone()))
                .unwrap_or_else(|| "My Walrus Site".to_string());

            persist_site_id_and_name(new_site_object_id, Some(name.clone()), ws_resources, path)?;
        }
    }
    Ok(())
}

/// Gets the configuration from the provided file, or looks in the default directory.
pub(crate) fn load_ws_resources(
    path: Option<&Path>,
    site_dir: &Path,
) -> Result<(Option<WSResources>, Option<PathBuf>)> {
    let default_paths = vec![site_dir.join(DEFAULT_WS_RESOURCES_FILE)];
    let path = path_or_defaults_if_exist(path, &default_paths);
    Ok((path.as_ref().map(WSResources::read).transpose()?, path))
}

// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::BTreeSet,
    num::NonZeroUsize,
    path::{Path, PathBuf},
    sync::mpsc::channel,
};

use anyhow::{anyhow, Result};
use notify::{RecursiveMode, Watcher};
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
    args::{PublishOptions, WalrusStoreOptions},
    backoff::ExponentialBackoffConfig,
    config::Config,
    display,
    preprocessor::Preprocessor,
    retry_client::RetriableSuiClient,
    site::{
        builder::SitePtb,
        config::WSResources,
        manager::SiteManager,
        resource::ResourceManager,
        RemoteSiteFactory,
        SITE_MODULE,
    },
    summary::{SiteDataDiffSummary, Summarizable},
    util::{
        get_site_id_from_response,
        id_to_base36,
        path_or_defaults_if_exist,
        persist_site_id_and_name,
        sign_and_send_ptb,
    },
};

const DEFAULT_WS_RESOURCES_FILE: &str = "ws-resources.json";

/// The continuous editing options.
#[derive(Debug, Clone)]
pub(crate) enum ContinuousEditing {
    /// Edit the site once and exit.
    Once,
    /// Watch the directory for changes and publish the site on change.
    Watch,
}

impl ContinuousEditing {
    /// Convert the flag to the enum.
    pub fn from_watch_flag(flag: bool) -> Self {
        if flag {
            ContinuousEditing::Watch
        } else {
            ContinuousEditing::Once
        }
    }
}

/// Options for the management of Walrus blobs.
#[derive(Debug, Clone)]
pub(crate) struct BlobManagementOptions {
    /// Forces a check of the expiration of all blobs, and extension if necessary.
    pub(crate) check_extend: bool,
}

impl BlobManagementOptions {
    /// Returns true if the expiration of all blobs should be checked.
    pub fn is_check_extend(&self) -> bool {
        self.check_extend
    }

    /// Returns an instance of `Self` with the expiration check disabled.
    pub fn no_status_check() -> Self {
        BlobManagementOptions {
            check_extend: false,
        }
    }
}

pub(crate) struct EditOptions {
    pub publish_options: PublishOptions,
    pub site_id: Option<ObjectID>,
    pub site_name: Option<String>,
    pub continuous_editing: ContinuousEditing,
    pub blob_options: BlobManagementOptions,
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
        continuous_editing: ContinuousEditing,
        blob_options: BlobManagementOptions,
    ) -> SiteEditor<EditOptions> {
        SiteEditor {
            context: self.context,
            config: self.config,
            edit_options: EditOptions {
                publish_options,
                site_id,
                site_name,
                continuous_editing,
                blob_options,
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

        let all_blobs = site
            .resources()
            .into_iter()
            .map(|resource| resource.info.blob_id)
            .collect::<Vec<_>>();

        tracing::debug!(?all_blobs, "retrieved the site for deletion");

        // Add warning if no deletable blobs found.
        if all_blobs.is_empty() {
            println!("Warning: No deletable resources found. This may be because the site was created with permanent=true");
        } else {
            // TODO: Change the site manager not to require the unnecessary info.
            let mut site_manager = SiteManager::new(
                self.config.clone(),
                Some(site_id),
                BlobManagementOptions::no_status_check(),
                WalrusStoreOptions::default(),
                None,
                None,
                NonZeroUsize::new(1).expect("non-zero"),
            )
            .await?;

            site_manager.delete_from_walrus(&all_blobs).await?;
        }

        // Delete objects on SUI blockchain
        let mut wallet = self.config.load_wallet()?;
        let ptb = SitePtb::new(self.config.package, Identifier::new(SITE_MODULE)?)?;
        let mut ptb = ptb.with_call_arg(&wallet.get_object_ref(site_id).await?.into())?;
        let site = RemoteSiteFactory::new(&retriable_client, self.config.package)
            .await?
            .get_from_chain(site_id)
            .await?;

        ptb.destroy(site.resources())?;
        let active_address = wallet.active_address()?;
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

    /// Run the editing operations requested.
    pub async fn run(&self) -> Result<()> {
        match self.edit_options.continuous_editing {
            ContinuousEditing::Once => self.run_single_and_print_summary().await?,
            ContinuousEditing::Watch => self.run_continuous().await?,
        }
        Ok(())
    }

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
            // self.edit_options.publish_options.max_concurrent,
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

    async fn run_single_and_print_summary(&self) -> Result<()> {
        let (active_address, response, summary) = self.run_single_edit().await?;
        print_summary(
            &self.config,
            &active_address,
            &self.edit_options.site_id,
            &response,
            &summary,
        )?;
        Ok(())
    }

    async fn run_continuous(&self) -> Result<()> {
        let (tx, rx) = channel();
        let mut watcher = notify::recommended_watcher(move |res| {
            tx.send(res).expect("Error in sending the watch event")
        })?;

        // Add a path to be watched. All files and directories at that path and
        // below will be monitored for changes.
        watcher.watch(self.directory(), RecursiveMode::Recursive)?;

        loop {
            match rx.recv() {
                Ok(event) => {
                    tracing::info!("change detected: {:?}", event);
                    self.run_single_and_print_summary().await?;
                }
                Err(e) => println!("Watch error!: {e}"),
            }
        }
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
                "error while processing the Sui transaction: {}",
                error
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
            );
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
            r#"To browse the site, run a testnet portal locally and visit:
    http://{base36_id}.localhost:3000

    (more info: https://docs.wal.app/walrus-sites/portal.html#running-the-portal-locally)"#,
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
///     or the site_name if a new site
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
    ws_resources: Option<WSResources>,
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

            let new_site_object_id = get_site_id_from_response(active_address, tx_effects);

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

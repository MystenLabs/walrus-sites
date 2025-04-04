// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

mod args;
mod backoff;
mod config;
mod display;
mod preprocessor;
mod publish;
mod retry_client;
mod site;
mod summary;
mod types;
mod util;
mod walrus;
use std::{num::NonZeroUsize, path::PathBuf};

use anyhow::{anyhow, Result};
use args::{Commands, GeneralArgs};
use backoff::ExponentialBackoffConfig;
use clap::Parser;
use config::Config;
use futures::TryFutureExt;
use publish::{load_ws_resources, ContinuousEditing, SiteEditor, WhenWalrusUpload};
use retry_client::RetriableSuiClient;
use site::{
    config::WSResources,
    manager::{SiteIdentifier, SiteManager},
    resource::ResourceManager,
    RemoteSiteFactory,
};
use util::path_or_defaults_if_exist;

use crate::{preprocessor::Preprocessor, util::id_to_base36};

/// The default path to the configuration file for the site builder.
const SITES_CONFIG_NAME: &str = "./sites-config.yaml";

// Define the `GIT_REVISION` and `VERSION` consts.
bin_version::bin_version!();

#[derive(Parser, Debug)]
#[clap(rename_all = "kebab-case", version = VERSION, propagate_version = true)]
struct Args {
    /// The path to the configuration file for the site builder.
    #[clap(short, long)]
    config: Option<PathBuf>,
    /// The context with which to load the configuration.
    ///
    /// If specified, the context will be taken from the config file. Otherwise, the default
    /// context, which is also specified in the config file, will be used.
    #[clap(long)]
    context: Option<String>,
    #[clap(flatten)]
    general: GeneralArgs,
    #[command(subcommand)]
    command: Commands,
}

/// Returns the default paths for the sites-config.yaml file.
pub fn sites_config_default_paths() -> Vec<PathBuf> {
    let mut default_paths = vec![SITES_CONFIG_NAME.into()];
    if let Ok(home_dir) = std::env::var("XDG_CONFIG_HOME") {
        default_paths.push(
            PathBuf::from(home_dir)
                .join("walrus")
                .join(SITES_CONFIG_NAME),
        );
    };
    if let Some(home_dir) = home::home_dir() {
        default_paths.push(
            home_dir
                .join(".config")
                .join("walrus")
                .join(SITES_CONFIG_NAME),
        );
    }
    default_paths
}

async fn run() -> Result<()> {
    tracing_subscriber::fmt::init();
    tracing::info!("initializing site builder");

    let args = Args::parse();
    tracing::info!(?args, "command line arguments");
    let config_path =
        path_or_defaults_if_exist(args.config.as_deref(), &sites_config_default_paths()).ok_or(
            anyhow!(
                "could not find a valid sites configuration file; \
            consider using  the --config flag to specify the config"
            ),
        )?;

    tracing::info!(?config_path, "loading sites configuration");
    let mut config = Config::load_from_multi_config(config_path, args.context.as_deref())?;
    tracing::debug!(?config, "configuration before merging");

    // Merge the configs and the CLI args. Serde default ensures that the `walrus_binary` and
    // `gas_budget` exist.
    config.merge(&args.general);
    tracing::info!(?config, "configuration loaded");

    match args.command {
        Commands::Publish {
            publish_options,
            site_name,
        } => {
            // Use the passed, name, or load the ws-resources file, if it exists, to take the site
            // name from it or use the default one.
            let (ws_resources, _) = load_ws_resources(
                publish_options.walrus_options.ws_resources.as_deref(),
                publish_options.directory.as_path(),
            )?;
            let site_name = site_name.unwrap_or_else(|| {
                ws_resources
                    .and_then(|res| res.site_name)
                    .unwrap_or_else(|| "Test Site".to_string())
            });

            SiteEditor::new(args.context, config)
                .with_edit_options(
                    publish_options,
                    SiteIdentifier::NewSite(site_name),
                    ContinuousEditing::Once,
                    WhenWalrusUpload::Modified,
                )
                .run()
                .await?
        }
        Commands::Update {
            publish_options,
            object_id,
            watch,
            force,
        } => {
            SiteEditor::new(args.context, config)
                .with_edit_options(
                    publish_options,
                    SiteIdentifier::ExistingSite(object_id),
                    ContinuousEditing::from_watch_flag(watch),
                    WhenWalrusUpload::from_force_flag(force),
                )
                .run()
                .await?
        }
        // Add a path to be watched. All files and directories at that path and
        // below will be monitored for changes.
        Commands::Sitemap { object } => {
            let all_dynamic_fields = RemoteSiteFactory::new(
                // TODO(giac): make the backoff configurable.
                &RetriableSuiClient::new_from_wallet(
                    &config.load_wallet()?,
                    ExponentialBackoffConfig::default(),
                )
                .await?,
                config.package,
            )
            .await?
            .get_existing_resources(object)
            .await?;
            println!("Pages in site at object id: {}", object);
            for (name, id) in all_dynamic_fields {
                println!("  - {:<40} {:?}", name, id);
            }
        }
        Commands::Convert { object_id } => println!("{}", id_to_base36(&object_id)?),
        Commands::ListDirectory { path } => {
            Preprocessor::preprocess(path.as_path())?;
        }
        Commands::Destroy { object } => {
            let site_editor = SiteEditor::new(args.context, config);
            site_editor.destroy(object).await?;
        }
        Commands::UpdateResource {
            resource,
            path,
            site_object,
            common,
        } => {
            let ws_res = common
                .ws_resources
                .clone()
                .as_ref()
                .map(WSResources::read)
                .transpose()?;
            let resource_manager = ResourceManager::new(
                config.walrus_client(),
                ws_res,
                common.ws_resources.clone(),
                None,
            )
            .await?;
            let resource = resource_manager
                .read_resource(&resource, path)
                .await?
                .ok_or(anyhow!(
                    "could not read the resource at path: {}",
                    resource.display()
                ))?;
            // TODO: make when upload configurable.
            let mut site_manager = SiteManager::new(
                config,
                SiteIdentifier::ExistingSite(site_object),
                WhenWalrusUpload::Always,
                common,
                None, // TODO: update the site metadata.
                NonZeroUsize::new(1).expect("non-zero"),
            )
            .await?;
            site_manager.update_single_resource(resource).await?;
            display::header("Resource updated successfully");
        }
    };

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    run()
        .inspect_err(|_| display::error("Error during execution"))
        .await
}

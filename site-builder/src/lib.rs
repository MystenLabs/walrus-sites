// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;

use anyhow::anyhow;
use args::{Args, Commands};
use config::Config;
use preprocessor::Preprocessor;
use publish::{load_ws_resources, SiteEditor};
use site::{
    config::WSResources,
    estimates::Estimator,
    manager::{SiteManager, BlobExtensions},
    quilts::QuiltsManager,
    resource::{ResourceManager, ResourceSet},
    SiteData,
    RemoteSiteFactory,
};
use sitemap::display_sitemap;
use util::{id_to_base36, path_or_defaults_if_exist};

pub mod args;
mod backoff;
pub mod config;
mod display;
mod preprocessor;
mod publish;
mod retry_client;
mod site;
// TODO(sew-251): This can be a standalone crate, helping integration testing and other projects
// using our contract.
pub use site::{config as site_config, contracts, resource::MAX_IDENTIFIER_SIZE};
mod sitemap;
mod suins;
mod summary;
// TODO(sew-251): This can be a standalone crate, helping integration testing and other projects
// using our contract.
pub mod types;
pub mod util;
mod walrus;

/// The default path to the configuration file for the site builder.
const SITES_CONFIG_NAME: &str = "./sites-config.yaml";

pub async fn run(args: Args) -> anyhow::Result<()> {
    run_internal(args)
        .await
        .inspect_err(|err| display::error(format!("Error during execution: {err}")))
}

// `Args` can currently only live in `main`, as it needs the `VERSION` constant which is created
// using `bin_version::bin_version!();` which can only run in `main`.
async fn run_internal(
    Args {
        config,
        context,
        general,
        command,
    }: Args,
) -> anyhow::Result<()> {
    let config_path = path_or_defaults_if_exist(config.as_deref(), &sites_config_default_paths())
        .ok_or(anyhow!(
        "could not find a valid sites configuration file; \
            consider using  the --config flag to specify the config"
    ))?;

    tracing::info!(?config_path, "loading sites configuration");
    let (mut config, selected_context) =
        Config::load_from_multi_config(config_path, context.as_deref())?;
    tracing::debug!(?config, "configuration before merging");

    // Merge the configs and the CLI args. Serde default ensures that the `walrus_binary` and
    // `gas_budget` exist.
    config.merge(&general);
    tracing::info!(?config, "configuration loaded");

    match command {
        Commands::Deploy {
            publish_options,
            site_name,
            object_id,
        } => {
            // Load the ws-resources file, to check for the site-object-id. If it exists, it means
            // the site is already deployed, in which case we should do update the site.
            // If it doesn't exist, we can publish a new site.
            let (ws_resources, _) = load_ws_resources(
                publish_options.walrus_options.ws_resources.as_deref(),
                publish_options.directory.as_path(),
            )?;

            // if `object_id` is Some use it, else use the one from the ws-resources file
            let site_object_id =
                object_id.or_else(|| ws_resources.as_ref().and_then(|res| res.object_id));

            SiteEditor::new(context, config)
                .with_edit_options(publish_options, site_object_id, site_name)
                .run_quilts()
                .await?
        }
        Commands::Publish {
            publish_options,
            site_name,
        } => {
            SiteEditor::new(context, config)
                .with_edit_options(publish_options, None, site_name)
                .run_quilts()
                .await?
        }
        Commands::Update {
            publish_options,
            object_id,
        } => {
            SiteEditor::new(context, config)
                .with_edit_options(publish_options, Some(object_id), None)
                .run_quilts()
                .await?
        }
        // Add a path to be watched. All files and directories at that path and
        // below will be monitored for changes.
        Commands::Sitemap { site_to_map } => {
            display_sitemap(site_to_map, selected_context, config).await?;
        }
        Commands::Convert { object_id } => println!("{}", id_to_base36(&object_id)?),
        Commands::ListDirectory { path, ws_resources } => {
            let (ws_resources_opt, _) = load_ws_resources(ws_resources.as_deref(), &path)?;
            let ws_res = ws_resources_opt;
            match ws_res {
                Some(ws_res) => {
                    Preprocessor::preprocess(path.as_path(), &ws_res.ignore)?;
                }
                None => {
                    Preprocessor::preprocess(path.as_path(), &None)?;
                }
            }
            display::header(format!(
                "Successfully preprocessed the {} directory!",
                path.display()
            ));
        }
        Commands::Destroy { object } => {
            let site_editor = SiteEditor::new(context, config);
            site_editor.destroy(object).await?;
        }
        // TODO(sew-495): Check whether quilts-extensions happen here.
        Commands::UpdateResources {
            resources,
            site_object,
            common,
        } => {
            let ws_res = common
                .ws_resources
                .clone()
                .as_ref()
                .map(WSResources::read)
                .transpose()?;
            let resource_manager = ResourceManager::new(ws_res, common.ws_resources.clone())?;
            let mut quilts_manager = QuiltsManager::new(config.walrus_client()).await?;
            let mut site_manager = SiteManager::new(
                config.clone(), // Clone to avoid moving config
                Some(site_object),
                None,
                None,
                Some(common.epoch_arg.clone()),
            )
            .await?;

            // Parse the resource paths into resource data
            let resource_data = resource_manager.parse_resources(resources)?;

            // If dry_run, show estimates using Estimator
            if common.dry_run {
                let estimator = Estimator::new();
                
                // Create mock resources for accurate Sui estimation (same as publish.rs)
                let chunks = quilts_manager.quilts_chunkify(
                    resource_data.clone(),
                    common.max_quilt_size,
                )?;
                let mock_resources = resource_manager.create_mock_resources_from_chunks(&chunks);
                
                // Combine mock resources with unchanged resources for accurate Sui estimation
                let mut mock_resource_set = ResourceSet::empty();
                mock_resource_set.extend(mock_resources);
                
                // Get existing site data for accurate estimation
                let existing_site = if site_manager.site_id.is_some() {
                    let retriable_client = site_manager.sui_client();
                    let package_id = config.package;
                    RemoteSiteFactory::new(retriable_client, package_id)
                        .await?
                        .get_from_chain(site_manager.site_id.unwrap())
                        .await?
                } else {
                    SiteData::empty()
                };
                
                let mock_local_site_data = resource_manager.to_site_data(mock_resource_set);
                
                // Calculate updates for Sui estimation
                let updates = mock_local_site_data.diff(&existing_site, BlobExtensions::Noop.into())?;
                
                // Show Walrus storage estimates
                estimator.show_walrus_estimates(
                    &mut quilts_manager,
                    resource_data.clone(),
                    common.epoch_arg.clone(),
                    common.max_quilt_size,
                    &BlobExtensions::Noop, // No extensions for update-resources
                ).await?;
                
                let walrus_package = site_manager.config.general.walrus_package.unwrap();
                
                // Show Sui gas estimates
                estimator.show_sui_gas_estimates(
                    &mut site_manager,
                    &updates,
                    BlobExtensions::Noop,
                    walrus_package,
                ).await?;
                
                println!("Dry run completed. No resources were actually stored or updated.");
                return Ok(());
            }

            // Store resources
            let stored_resources = quilts_manager
                .store_quilts(
                    resource_data,
                    common.epoch_arg,
                    common.max_quilt_size,
                )
                .await?;

            // Convert to ResourceSet for the site manager
            let mut resource_set = ResourceSet::empty();
            resource_set.extend(stored_resources);

            // TODO(sew-604): Extend the lifetime of the rest of the resources that belong to the
            // site?
            site_manager.update_resources(resource_set).await?;
        }
    };

    Ok(())
}

/// Returns the default paths for the sites-config.yaml file.
fn sites_config_default_paths() -> Vec<PathBuf> {
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

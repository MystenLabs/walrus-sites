// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{num::NonZeroUsize, path::PathBuf};

use anyhow::anyhow;
use args::{Args, Commands};
use config::Config;
use preprocessor::Preprocessor;
use publish::{load_ws_resources, SiteEditor};
use site::{config::WSResources, manager::SiteManager, resource::ResourceManager};
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
// TODO: This can be a standalone crate, helping integration testing and other projects using our
// contract.
pub use site::{config as site_config, contracts, resource::MAX_IDENTIFIER_SIZE};
mod sitemap;
mod suins;
mod summary;
// TODO: This can be a standalone crate, helping integration testing and other projects using our
// contract.
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
        Commands::DeployQuilts {
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
        Commands::PublishQuilts {
            publish_options,
            site_name,
        } => {
            SiteEditor::new(context, config)
                .with_edit_options(publish_options, None, site_name)
                .run_quilts()
                .await?
        }
        Commands::UpdateQuilts {
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
            let resource_manager =
                ResourceManager::new(config.walrus_client(), ws_res, common.ws_resources.clone())
                    .await?;
            let resource = resource_manager
                .read_single_blob_resource(&resource, path)
                .await?
                .ok_or(anyhow!(
                    "could not read the resource at path: {}",
                    resource.display()
                ))?;
            let mut site_manager = SiteManager::new(
                config,
                Some(site_object),
                common,
                None,
                None,
                NonZeroUsize::new(1).expect("non-zero"),
            )
            .await?;
            site_manager.update_single_resource(resource).await?;
            display::header("Resource updated successfully");
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

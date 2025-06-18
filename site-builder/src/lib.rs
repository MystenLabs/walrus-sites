use std::{num::NonZeroUsize, path::PathBuf};

use anyhow::anyhow;
use args::{Commands, GeneralArgs};
use config::Config;
use preprocessor::Preprocessor;
use publish::{load_ws_resources, BlobManagementOptions, ContinuousEditing, SiteEditor};
use site::{
    config::WSResources,
    manager::{SiteIdentifier, SiteManager},
    resource::ResourceManager,
};
use sitemap::display_sitemap;
use util::{id_to_base36, path_or_defaults_if_exist};

pub mod args;
mod backoff;
pub mod config;
mod display;
mod network;
mod preprocessor;
mod publish;
mod retry_client;
mod site;
mod sitemap;
mod suins;
mod summary;
mod types;
mod util;
mod walrus;

/// The default path to the configuration file for the site builder.
const SITES_CONFIG_NAME: &str = "./sites-config.yaml";

pub async fn run(
    config: Option<PathBuf>,
    context: Option<String>,
    general: GeneralArgs,
    command: Commands,
) -> anyhow::Result<()> {
    run_internal(config, context, general, command)
        .await
        .inspect_err(|_| display::error("Error during execution"))
}

// `Args` can currently only live in `main`, as it needs the `VERSION` constant which is created
// using `bin_version::bin_version!();` which can only run in `main`.
async fn run_internal(
    config: Option<PathBuf>,
    // The context with which to load the configuration.
    //
    // If specified, the context will be taken from the config file. Otherwise, the default
    // context, which is also specified in the config file, will be used.
    context: Option<String>,
    general: GeneralArgs,
    command: Commands,
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
            watch,
            check_extend,
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

            let (identifier, continuous_editing, blob_management) = match site_object_id {
                Some(object_id) => (
                    SiteIdentifier::ExistingSite(object_id),
                    ContinuousEditing::from_watch_flag(watch),
                    BlobManagementOptions { check_extend },
                ),
                None => (
                    SiteIdentifier::NewSite(site_name.unwrap_or_else(|| {
                        ws_resources
                            .and_then(|res| res.site_name)
                            .unwrap_or_else(|| "My Walrus Site".to_string())
                    })),
                    ContinuousEditing::Once,
                    BlobManagementOptions::no_status_check(),
                ),
            };

            SiteEditor::new(context, config)
                .with_edit_options(
                    publish_options,
                    identifier,
                    continuous_editing,
                    blob_management,
                )
                .run()
                .await?
        }
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
                    .unwrap_or_else(|| "My Walrus Site".to_string())
            });

            SiteEditor::new(context, config)
                .with_edit_options(
                    publish_options,
                    SiteIdentifier::NewSite(site_name),
                    ContinuousEditing::Once,
                    BlobManagementOptions::no_status_check(),
                )
                .run()
                .await?
        }
        #[allow(deprecated)]
        Commands::Update {
            publish_options,
            object_id,
            watch,
            force,
            check_extend,
        } => {
            if force {
                display::warning(
                    "Warning: The --force flag is deprecated and will be removed in a future \
                    version. Please use --check-extend instead.",
                )
            }
            SiteEditor::new(context, config)
                .with_edit_options(
                    publish_options,
                    SiteIdentifier::ExistingSite(object_id),
                    ContinuousEditing::from_watch_flag(watch),
                    // Check the extension if either `check_extend` is true or `force` is true.
                    // This is for backwards compatibility.
                    // TODO: Remove once the `force` flag is deprecated.
                    BlobManagementOptions {
                        check_extend: check_extend || force,
                    },
                )
                .run()
                .await?
        }
        // Add a path to be watched. All files and directories at that path and
        // below will be monitored for changes.
        Commands::Sitemap { site_to_map } => {
            display_sitemap(site_to_map, selected_context, config).await?;
        }
        Commands::Convert { object_id } => println!("{}", id_to_base36(&object_id)?),
        Commands::ListDirectory { path } => {
            Preprocessor::preprocess(path.as_path())?;
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
                BlobManagementOptions::no_status_check(),
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

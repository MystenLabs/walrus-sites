// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

mod backoff;
mod display;
mod preprocessor;
mod publish;
mod retry_client;
mod site;
mod summary;
mod types;
mod util;
mod walrus;
use std::{
    collections::HashMap,
    num::{NonZeroU32, NonZeroUsize},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, ensure, Result};
use backoff::ExponentialBackoffConfig;
use clap::{Parser, Subcommand};
use futures::TryFutureExt;
use publish::{ContinuousEditing, SiteEditor, WhenWalrusUpload};
use retry_client::RetriableSuiClient;
use serde::{Deserialize, Serialize};
use site::{
    config::WSResources,
    manager::{SiteIdentifier, SiteManager},
    resource::ResourceManager,
    RemoteSiteFactory,
};
use sui_sdk::wallet_context::WalletContext;
use sui_types::base_types::ObjectID;
use util::path_or_defaults_if_exist;
use walrus::{output::EpochCount, Walrus};

use crate::{
    preprocessor::Preprocessor,
    util::{id_to_base36, load_wallet_context},
};

// Define the `GIT_REVISION` and `VERSION` consts.
bin_version::bin_version!();

const SITES_CONFIG_NAME: &str = "./sites-config.yaml";

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

#[derive(Parser, Clone, Debug, Deserialize)]
#[clap(rename_all = "kebab-case")]
pub(crate) struct GeneralArgs {
    /// The URL or the RPC endpoint to connect the client to.
    ///
    /// Can be specified as a CLI argument or in the config.
    #[clap(long)]
    rpc_url: Option<String>,
    /// The path to the Sui Wallet config.
    ///
    /// Can be specified as a CLI argument or in the config.
    #[clap(long)]
    wallet: Option<PathBuf>,
    /// The path or name of the walrus binary.
    ///
    /// The Walrus binary will then be called with this configuration to perform actions on Walrus.
    /// Can be specified as a CLI argument or in the config.
    #[clap(long)]
    #[serde(default = "default::walrus_binary")]
    walrus_binary: Option<String>,
    /// The path to the configuration for the Walrus client.
    ///
    /// This will be passed to the calls to the Walrus binary.
    /// Can be specified as a CLI argument or in the config.
    #[clap(long)]
    walrus_config: Option<PathBuf>,
    /// The gas budget for the operations on Sui.
    ///
    /// Can be specified as a CLI argument or in the config.
    #[clap(long)]
    #[clap(short, long)]
    #[serde(default = "default::gas_budget")]
    gas_budget: Option<u64>,
}

impl Default for GeneralArgs {
    fn default() -> Self {
        Self {
            rpc_url: None,
            wallet: None,
            walrus_binary: default::walrus_binary(),
            walrus_config: None,
            gas_budget: default::gas_budget(),
        }
    }
}

macro_rules! merge {
    ($self:ident, $other:ident, $field:ident) => {
        if $other.$field.is_some() {
            $self.$field = $other.$field.clone();
        }
    };
}

macro_rules! merge_fields {
    ($self:ident, $other:ident, $($field:ident),* $(,)?) => (
        $(
            merge!($self, $other, $field);
        )*
    );
}

impl GeneralArgs {
    /// Merges two instances of [`GeneralArgs`], keeping all the `Some` values.
    ///
    /// The values of `other` are taken before the values of `self`.
    pub fn merge(&mut self, other: &Self) {
        merge_fields!(
            self,
            other,
            rpc_url,
            wallet,
            walrus_binary,
            walrus_config,
            gas_budget,
        );
    }
}

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case")]
enum Commands {
    /// Publish a new site on Sui.
    Publish {
        #[clap(flatten)]
        publish_options: PublishOptions,
        /// The name of the site.
        #[clap(short, long, default_value = "test site")]
        site_name: String,
    },
    /// Update an existing site.
    Update {
        #[clap(flatten)]
        publish_options: PublishOptions,
        /// The object ID of a partially published site to be completed.
        object_id: ObjectID,
        #[clap(short, long, action)]
        watch: bool,
        /// Publish all resources to Sui and Walrus, even if they may be already present.
        ///
        /// This can be useful in case the Walrus devnet is reset, but the resources are still
        /// available on Sui.
        #[clap(long, action)]
        force: bool,
    },
    /// Convert an object ID in hex format to the equivalent Base36 format.
    ///
    /// This command may be useful to browse a site, given it object ID.
    Convert {
        /// The object id (in hex format) to convert
        object_id: ObjectID,
    },
    /// Show the pages composing the site at the given object ID.
    Sitemap { object: ObjectID },
    /// Preprocess the directory, creating and linking index files.
    /// This command allows to publish directories as sites. Warning: Rewrites all `index.html`
    /// files.
    ListDirectory { path: PathBuf },
    /// Completely destroys the site at the given object id.
    ///
    /// Removes all resources and routes, and destroys the site, returning the Sui storage rebate to
    /// the owner. Warning: this action is irreversible! Re-publishing the site will generate a
    /// different Site object ID.
    Destroy { object: ObjectID },
    /// Adds or updates a single resource in a site, eventually replacing any pre-existing ones.
    ///
    /// The ws_resource file will still be used to determine the resource's headers.
    UpdateResource {
        /// The path to the resource to be added.
        #[clap(long)]
        resource: PathBuf,
        /// The path the resource should have in the site.
        ///
        /// Should be in the form `/path/to/resource.html`, with a leading `/`.
        #[clap(long)]
        path: String,
        /// The object ID of the Site object on Sui, to which the resource will be added.
        #[clap(long)]
        site_object: ObjectID,
        /// The path to the Walrus sites resources file.
        ///
        /// This JSON configuration file defined HTTP resource headers and other utilities for your
        /// files. By default, the file is expected to be named `ws-resources.json` and located in the
        /// root of the site directory.
        ///
        /// The configuration file _will not_ be uploaded to Walrus.
        #[clap(long)]
        // TODO: deduplicate with the `publish_options` in the `Publish` and `Update` commands.
        ws_resources: Option<PathBuf>,
        /// The number of epochs for which to save the resources on Walrus.
        ///
        /// If set to `max`, the resources are stored for the maximum number of epochs allowed on
        /// Walrus. Otherwise, the resources are stored for the specified number of epochs. The
        /// number of epochs must be greater than 0.
        #[clap(long, value_parser = EpochCountOrMax::parse_epoch_count)]
        epochs: EpochCountOrMax,
        /// By default, sites are deletable with site-builder delete command. By passing --permanent, the site is deleted only after `epochs` expiration.
        /// Make resources permanent (non-deletable)
        #[clap(long, action = clap::ArgAction::SetTrue)]
        permanent: bool,
        /// Perform a dry run (you'll be asked for confirmation before committing changes).
        #[clap(long)]
        dry_run: bool,
    },
}

#[derive(Parser, Debug, Clone)]
pub struct PublishOptions {
    /// The directory containing the site sources.
    pub directory: PathBuf,
    /// The path to the Walrus sites resources file.
    ///
    /// This JSON configuration file defined HTTP resource headers and other utilities for your
    /// files. By default, the file is expected to be named `ws-resources.json` and located in the
    /// root of the site directory.
    ///
    /// The configuration file _will not_ be uploaded to Walrus.
    #[clap(long)]
    ws_resources: Option<PathBuf>,
    /// The number of epochs for which to save the resources on Walrus.
    ///
    /// If set to `max`, the resources are stored for the maximum number of epochs allowed on
    /// Walrus. Otherwise, the resources are stored for the specified number of epochs. The
    /// number of epochs must be greater than 0.
    #[clap(long, value_parser = EpochCountOrMax::parse_epoch_count)]
    pub epochs: EpochCountOrMax,
    /// Preprocess the directory before publishing.
    /// See the `list-directory` command. Warning: Rewrites all `index.html` files.
    #[clap(long, action)]
    pub list_directory: bool,
    /// The maximum number of concurrent calls to the Walrus CLI for the computation of blob IDs.
    #[clap(long)]
    max_concurrent: Option<NonZeroUsize>,
    /// By default, sites are deletable with site-builder delete command. By passing --permanent, the site is deleted only after `epochs` expiration.
    /// Make resources permanent (non-deletable)
    #[clap(long, action = clap::ArgAction::SetTrue)]
    permanent: bool,
    /// Perform a dry run (you'll be asked for confirmation before committing changes).
    #[clap(long)]
    dry_run: bool,
}

/// Configuration for the site builder, complete with separate context for networks.
#[derive(Deserialize, Debug, Clone)]
pub(crate) struct MultiConfig {
    pub contexts: HashMap<String, Config>,
    pub default_context: String,
}

/// The configuration for the site builder.
#[derive(Deserialize, Debug, Clone)]
pub(crate) struct Config {
    #[serde(default = "default::default_portal")]
    pub portal: String,
    pub package: ObjectID,
    #[serde(default)]
    pub general: GeneralArgs,
}

impl Config {
    pub fn load_multi_config(path: impl AsRef<Path>, context: Option<&str>) -> Result<Self> {
        let mut multi_config =
            serde_yaml::from_str::<MultiConfig>(&std::fs::read_to_string(path)?)?;

        let context = context.unwrap_or_else(|| &multi_config.default_context);
        tracing::info!(?context, "using context");

        let config = multi_config
            .contexts
            .remove(context)
            .ok_or_else(|| anyhow!("could not find the context: {}", context))?;

        if context != multi_config.default_context {
            display::warn(format!(
                "using a non-default context ({}); the default context is: {}\n\
                Please ensure that this matches your wallet and Walrus CLI context\n",
                context, multi_config.default_context,
            ));
        }

        Ok(config)
    }

    /// Merges the other [`GeneralArgs`] (taken from the CLI) with the `general` in the struct.
    ///
    /// The values in `other_general` take precedence.
    pub fn merge(&mut self, other_general: &GeneralArgs) {
        self.general.merge(other_general);
    }

    pub fn walrus_binary(&self) -> String {
        self.general
            .walrus_binary
            .as_ref()
            .expect("serde default => binary exists")
            .to_owned()
    }

    pub fn gas_budget(&self) -> u64 {
        self.general
            .gas_budget
            .expect("serde default => gas budget exists")
    }

    /// Creates a Walrus client with the configuration from `self`.
    pub fn walrus_client(&self) -> Walrus {
        Walrus::new(
            self.walrus_binary(),
            self.gas_budget(),
            self.general.rpc_url.clone(),
            self.general.walrus_config.clone(),
            self.general.wallet.clone(),
        )
    }

    /// Returns a [`WalletContext`] from the configuration.
    pub fn wallet(&self) -> Result<WalletContext> {
        load_wallet_context(&self.general.wallet)
    }
}

/// The number of epochs to store the blobs for.
///
/// Can be either a non-zero number of epochs or the special value `max`, which will store the blobs
/// for the maximum number of epochs allowed by the system object on chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EpochCountOrMax {
    /// Store the blobs for the maximum number of epochs allowed.
    #[serde(rename = "max")]
    Max,
    /// The number of epochs to store the blobs for.
    #[serde(untagged)]
    Epochs(NonZeroU32),
}

impl EpochCountOrMax {
    fn parse_epoch_count(input: &str) -> Result<Self> {
        if input == "max" {
            Ok(Self::Max)
        } else {
            let epochs = input.parse::<u32>()?;
            Ok(Self::Epochs(NonZeroU32::new(epochs).ok_or_else(|| {
                anyhow!("invalid epoch count; please a number >0 or `max`")
            })?))
        }
    }

    /// Tries to convert the `EpochCountOrMax` into an `EpochCount` value.
    ///
    /// If the `EpochCountOrMax` is `Max`, the `max_epochs_ahead` is used as the maximum number of
    /// epochs that can be stored ahead.
    pub fn try_into_epoch_count(&self, max_epochs_ahead: EpochCount) -> anyhow::Result<EpochCount> {
        match self {
            EpochCountOrMax::Max => Ok(max_epochs_ahead),
            EpochCountOrMax::Epochs(epochs) => {
                let epochs = epochs.get();
                ensure!(
                    epochs <= max_epochs_ahead,
                    "blobs can only be stored for up to {} epochs ahead; {} epochs were requested",
                    max_epochs_ahead,
                    epochs
                );
                Ok(epochs)
            }
        }
    }
}

mod default {
    pub(crate) fn walrus_binary() -> Option<String> {
        Some("walrus".to_owned())
    }
    pub(crate) fn gas_budget() -> Option<u64> {
        Some(500_000_000)
    }

    pub(crate) fn default_portal() -> String {
        "wal.app".to_owned()
    }
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
    let config_path = path_or_defaults_if_exist(&args.config, &sites_config_default_paths())
        .ok_or(anyhow!(
            "could not find a valid sites configuration file; \
            consider using  the --config flag to specify the config"
        ))?;
    tracing::info!(?config_path, "loading sites configuration");
    let mut config = Config::load_multi_config(config_path, args.context.as_deref())?;

    // Merge the configs and the CLI args. Serde default ensures that the `walrus_binary` and
    // `gas_budget` exist.
    config.merge(&args.general);
    tracing::info!(?config, "configuration loaded");

    match args.command {
        Commands::Publish {
            publish_options,
            site_name,
        } => {
            SiteEditor::new(config)
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
            SiteEditor::new(config)
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
                    &config.wallet()?,
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
            let site_editor = SiteEditor::new(config);
            site_editor.destroy(object).await?;
        }
        Commands::UpdateResource {
            resource,
            path,
            site_object,
            ws_resources,
            epochs,
            permanent,
            dry_run,
        } => {
            let ws_res = ws_resources
                .clone()
                .as_ref()
                .map(WSResources::read)
                .transpose()?;
            let resource_manager =
                ResourceManager::new(config.walrus_client(), ws_res, ws_resources, None).await?;
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
                epochs,
                WhenWalrusUpload::Always,
                permanent,
                dry_run,
                None, // TODO: update the site metadata.
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

// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

mod display;
mod preprocessor;
mod publish;
mod site;
mod summary;
mod types;
mod util;
mod walrus;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use futures::TryFutureExt;
use publish::{ContinuousEditing, PublishOptions, SiteEditor, WhenWalrusUpload};
use serde::Deserialize;
use site::{manager::SiteIdentifier, RemoteSiteFactory};
use sui_types::base_types::ObjectID;
use util::path_or_defaults_if_exist;

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
}

mod default {
    pub(crate) fn walrus_binary() -> Option<String> {
        Some("walrus".to_owned())
    }
    pub(crate) fn gas_budget() -> Option<u64> {
        Some(500_000_000)
    }

    pub(crate) fn default_portal() -> String {
        "walrus.site".to_owned()
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
    let mut config = serde_yaml::from_str::<Config>(&std::fs::read_to_string(config_path)?)?;

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
            let wallet = load_wallet_context(&config.general.wallet)?;
            let all_dynamic_fields =
                RemoteSiteFactory::new(&wallet.get_client().await?, config.package)
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
    };

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    run()
        .inspect_err(|_| display::error("Error during execution"))
        .await
}

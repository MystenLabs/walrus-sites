// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

mod display;
mod preprocessor;
mod publish;
mod site;
mod util;
mod walrus;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use futures::TryFutureExt;
use publish::{publish_site, update_site};
use serde::Deserialize;
use site::content::ContentEncoding;
use sui_types::base_types::ObjectID;

use crate::{
    preprocessor::Preprocessor,
    util::{get_existing_resource_ids, id_to_base36, load_wallet_context},
};

#[derive(Parser, Debug)]
#[clap(rename_all = "kebab-case")]
struct Args {
    /// The path to the configuration file for the site builder.
    #[clap(short, long, default_value = "builder.yaml")]
    config: PathBuf,
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
        /// The directory containing the site sources.
        directory: PathBuf,
        /// The encoding for the contents of the site's resources.
        #[clap(short = 'e', long, value_enum, default_value_t = ContentEncoding::PlainText)]
        content_encoding: ContentEncoding,
        /// The name of the site.
        #[clap(short, long, default_value = "test site")]
        site_name: String,
        /// The number of epochs for which to save the resources on Walrus.
        #[clap(long, default_value_t = 1)]
        epochs: u64,
        /// Preprocess the directory before publishing.
        /// See the `list-directory` command. Warning: Rewrites all `index.html` files.
        #[clap(long, action)]
        list_directory: bool,
    },
    /// Update an existing site
    Update {
        /// The directory containing the site sources.
        directory: PathBuf,
        /// The object ID of a partially published site to be completed.
        object_id: ObjectID,
        /// The encoding for the contents of the site's resources.
        #[clap(short = 'e', long, value_enum, default_value_t = ContentEncoding::PlainText)]
        content_encoding: ContentEncoding,
        #[clap(short, long, action)]
        watch: bool,
        /// The number of epochs for which to save the updated resources on Walrus.
        #[clap(long, default_value_t = 1)]
        epochs: u64,
        /// Publish all resources to Sui and Walrus, even if they may be already present.
        ///
        /// This can be useful in case the Walrus devnet is reset, but the resources are still
        /// available on Sui.
        #[clap(long, action)]
        force: bool,
        /// Preprocess the directory before updating.
        /// See the `list-directory` command. Warning: Rewrites all `index.html` files.
        #[clap(long, action)]
        list_directory: bool,
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

async fn run() -> Result<()> {
    tracing_subscriber::fmt::init();
    tracing::info!("initializing site builder");

    let args = Args::parse();
    let mut config: Config = std::fs::read_to_string(&args.config)
        .context(format!(
            "unable to read config {:?}; consider using the --config flag to point to the config",
            args.config
        ))
        .and_then(|s| {
            serde_yaml::from_str(&s)
                .context(format!("unable to parse yaml in file {:?}", args.config))
        })?;
    // Merge the configs and the CLI args. Serde default ensures that the `walrus_binary` and
    // `gas_budget` exist.
    config.merge(&args.general);
    tracing::info!(?config, "configuration loaded");

    match &args.command {
        Commands::Publish {
            directory,
            content_encoding,
            site_name,
            epochs,
            list_directory,
        } => {
            publish_site(
                directory,
                content_encoding,
                site_name,
                &config,
                *epochs,
                *list_directory,
            )
            .await?
        }
        Commands::Update {
            directory,
            object_id,
            content_encoding,
            watch,
            epochs,
            force,
            list_directory,
        } => {
            update_site(
                directory,
                content_encoding,
                object_id,
                &config,
                *watch,
                *epochs,
                *force,
                *list_directory,
            )
            .await?;
        }
        // Add a path to be watched. All files and directories at that path and
        // below will be monitored for changes.
        Commands::Sitemap { object } => {
            let wallet = load_wallet_context(&config.general.wallet)?;
            let all_dynamic_fields =
                get_existing_resource_ids(&wallet.get_client().await?, *object).await?;
            println!("Pages in site at object id: {}", object);
            for (name, id) in all_dynamic_fields {
                println!("  - {:<40} {:?}", name, id);
            }
        }
        Commands::Convert { object_id } => println!("{}", id_to_base36(object_id)?),
        Commands::ListDirectory { path } => {
            Preprocessor::preprocess(path)?;
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

// mod network;
mod publish;
mod site;
mod util;
mod walrus;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use publish::{publish_site, update_site};
use serde::Deserialize;
use site::content::ContentEncoding;
use sui_types::{base_types::ObjectID, Identifier};
use walrus_service::{cli_utils::load_wallet_context, client::Config as WalrusConfig};

use crate::util::{get_existing_resource_ids, id_to_base36};

#[derive(Parser, Debug)]
#[clap(rename_all = "kebab-case")]
struct Args {
    #[arg(short, long, default_value = "config.yaml")]
    config: PathBuf,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case")]
enum Commands {
    /// Publish a new site on Sui.
    Publish {
        /// The directory containing the site sources.
        directory: PathBuf,
        /// The encoding for the contents of the BlockPages.
        #[clap(short = 'e', long, value_enum, default_value_t = ContentEncoding::Gzip)]
        content_encoding: ContentEncoding,
        /// The name of the BlockSite.
        #[clap(short, long, default_value = "test site")]
        site_name: String,
        /// The number of epochs for which to save the resources on Walrus.
        #[clap(long, default_value_t = 1)]
        epochs: u64,
    },
    /// Update an existing site
    Update {
        /// The directory containing the site sources.
        directory: PathBuf,
        /// The object ID of a partially published site to be completed.
        object_id: ObjectID,
        /// The encoding for the contents of the BlockPages.
        #[clap(short = 'e', long, value_enum, default_value_t = ContentEncoding::Gzip)]
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
    },
    /// Convert an object ID in hex format to the equivalent Base36 format.
    ///
    /// This command may be useful to browse a site, given it object ID.
    Convert {
        /// The object id (in hex format) to convert
        object_id: ObjectID,
    },
    /// Show the pages composing the blocksite at the given object ID.
    Sitemap { object: ObjectID },
}

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct Config {
    #[serde(default = "blocksite_module")]
    pub module: Identifier,
    #[serde(default = "default_portal")]
    pub portal: String,
    pub package: ObjectID,
    pub gas_coin: ObjectID,
    pub gas_budget: u64,
    pub walrus: WalrusConfig,
}

fn blocksite_module() -> Identifier {
    Identifier::new("blocksite").expect("valid literal identifier")
}

fn default_portal() -> String {
    "blocksite.net".to_owned()
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let config: Config = std::fs::read_to_string(&args.config)
        .context(format!("unable to read config file {:?}", args.config))
        .and_then(|s| {
            serde_yaml::from_str(&s)
                .context(format!("unable to parse toml in file {:?}", args.config))
        })?;

    match &args.command {
        Commands::Publish {
            directory,
            content_encoding,
            site_name,
            epochs,
        } => publish_site(directory, content_encoding, site_name, &config, *epochs).await?,
        Commands::Update {
            directory,
            object_id,
            content_encoding,
            watch,
            epochs,
            force,
        } => {
            update_site(
                directory,
                content_encoding,
                object_id,
                &config,
                *watch,
                *epochs,
                *force,
            )
            .await?;
        }
        // Add a path to be watched. All files and directories at that path and
        // below will be monitored for changes.
        Commands::Sitemap { object } => {
            let wallet = load_wallet_context(&config.walrus.wallet_config.clone())?;
            let all_dynamic_fields =
                get_existing_resource_ids(&wallet.get_client().await?, *object).await?;
            println!("Pages in site at object id: {}", object);
            for (name, id) in all_dynamic_fields {
                println!("  - {:<40} {:?}", name, id);
            }
        }
        Commands::Convert { object_id } => println!("{}", id_to_base36(object_id)?),
    };

    Ok(())
}

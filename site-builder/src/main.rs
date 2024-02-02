mod calls;
mod manager;
mod network;
mod page;
mod util;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use network::Network;
use page::ContentEncoding;
use serde::Deserialize;
use sui_sdk::rpc_types::{SuiTransactionBlockEffects, SuiTransactionBlockEffectsAPI};
use sui_types::{
    base_types::{ObjectID, SuiAddress},
    Identifier,
};

use crate::{
    manager::SuiManager,
    page::{Page, Site},
    util::{id_to_base36, recursive_readdir},
};

#[derive(Parser, Debug)]
#[clap(rename_all = "kebab-case")]
struct Args {
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case")]
enum Commands {
    Publish {
        directory: PathBuf,
        #[clap(short = 'e', long, value_enum, default_value_t = ContentEncoding::PlainText)]
        content_encoding: ContentEncoding,
        #[arg(short, long, default_value = "test site")]
        site_name: String,
    },
    Convert {
        object_id: ObjectID,
    },
}

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub address: SuiAddress,
    pub keystore: PathBuf,
    pub module: Identifier,
    pub package: ObjectID,
    pub network: Network,
    pub gas_coin: ObjectID,
    pub gas_budget: u64,
}

fn print_effects(config: &Config, site_name: &str, effects: &SuiTransactionBlockEffects) {
    println!("\n# Effects");
    let created_id = effects
        .created()
        .iter()
        .find(|c| c.owner == config.address)
        .expect("Could not find the object ID for the created blocksite.")
        .reference
        .object_id;
    println!("New blocksite '{}' created: {}", site_name, created_id);
    let base36 = id_to_base36(&created_id).expect("Could not convert the id to base 36.");
    println!(
        "Find it at https://{}.blocksite.net\nor http://{}.localhost:8000",
        &base36, &base36,
    );
    println!("Gas cost summary (MIST):");
    let summary = effects.gas_cost_summary();
    println!("   - Computation: {}", summary.computation_cost);
    println!("   - Storage: {}", summary.storage_cost);
    println!("   - Storage rebate: {}", summary.storage_rebate);
    println!(
        "   - Non refundable storage: {}",
        summary.non_refundable_storage_fee
    );

    println!(
        "For a total cost of: {} SUI",
        (summary.computation_cost + summary.storage_cost - summary.storage_rebate) as f64 / 1e9
    )
}

async fn publish(
    directory: &PathBuf,
    content_encoding: &ContentEncoding,
    site_name: &str,
    config: &Config,
) -> Result<()> {
    let pages = recursive_readdir(directory)
        .iter()
        .map(|f| Page::read(f, directory, content_encoding))
        .collect::<Result<Vec<_>>>()?;

    let mut manager = SuiManager::new(config.clone()).await?;
    let response = manager.publish_site(&Site::new(site_name), &pages).await?;
    let effects = response.effects.unwrap();
    print_effects(config, site_name, &effects);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let config: Config = toml::from_str(&std::fs::read_to_string(&args.config)?)?;
    match &args.command {
        Commands::Publish {
            directory,
            content_encoding,
            site_name,
        } => publish(directory, content_encoding, site_name, &config).await?,
        Commands::Convert { object_id } => {
            println!("{}", id_to_base36(object_id)?)
        }
    };
    Ok(())
}

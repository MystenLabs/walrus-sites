use std::{path::Path, sync::mpsc::channel};

use anyhow::Result;
use notify::{RecursiveMode, Watcher};
use sui_sdk::rpc_types::SuiTransactionBlockResponse;
use sui_types::base_types::{ObjectID, SuiAddress};
use walrus_service::{cli_utils::load_wallet_context, client::Client as WalrusClient};
use walrus_sui::client::SuiContractClient;

use crate::{
    site::{
        content::ContentEncoding,
        manager::{SiteIdentifier, SiteManager},
        // manager::SiteManager,
        resource::{OperationsSummary, ResourceManager},
    },
    util::{get_site_id_from_response, id_to_base36},
    Config,
};

pub async fn publish_site(
    directory: &Path,
    content_encoding: &ContentEncoding,
    site_name: &str,
    config: &Config,
    epochs: u64,
) -> Result<()> {
    edit_site(
        directory,
        content_encoding,
        SiteIdentifier::NewSite(site_name.to_owned()),
        config,
        epochs,
        false,
    )
    .await
}

pub async fn watch_edit_site(
    directory: &Path,
    content_encoding: &ContentEncoding,
    site_id: SiteIdentifier,
    config: &Config,
    epochs: u64,
    force: bool,
) -> Result<()> {
    let (tx, rx) = channel();
    let mut watcher = notify::recommended_watcher(move |res| {
        tx.send(res).expect("Error in sending the watch event")
    })?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(directory, RecursiveMode::Recursive)?;

    loop {
        match rx.recv() {
            Ok(event) => {
                tracing::info!("change detected: {:?}", event);
                edit_site(
                    directory,
                    content_encoding,
                    site_id.clone(),
                    config,
                    epochs,
                    force,
                )
                .await?;
            }
            Err(e) => println!("Watch error!: {}", e),
        }
    }
}

pub async fn update_site(
    directory: &Path,
    content_encoding: &ContentEncoding,
    site_object: &ObjectID,
    config: &Config,
    watch: bool,
    epochs: u64,
    force: bool,
) -> Result<()> {
    if watch {
        watch_edit_site(
            directory,
            content_encoding,
            SiteIdentifier::ExistingSite(*site_object),
            config,
            epochs,
            force,
        )
        .await
    } else {
        edit_site(
            directory,
            content_encoding,
            SiteIdentifier::ExistingSite(*site_object),
            config,
            epochs,
            force,
        )
        .await
    }
}

pub async fn edit_site(
    directory: &Path,
    content_encoding: &ContentEncoding,
    site_id: SiteIdentifier,
    config: &Config,
    epochs: u64,
    force: bool,
) -> Result<()> {
    tracing::debug!(?site_id, ?directory, "editing site");
    let wallet = load_wallet_context(&config.walrus.wallet_config.clone())?;

    let contract_client = SuiContractClient::new(
        wallet,
        config.walrus.system_pkg,
        config.walrus.system_object,
        config.gas_budget,
    )
    .await?;
    let walrus_client = WalrusClient::new(config.walrus.clone(), contract_client).await?;

    let mut resource_manager = ResourceManager::new()?;
    resource_manager.read_dir(directory, content_encoding, walrus_client.encoding_config())?;
    tracing::debug!(resources=%resource_manager.resources, "resources loaded from directory");

    let site_manager = SiteManager::new(config, walrus_client, site_id.clone(), epochs, force).await?;
    let (response, summary) = site_manager.update_site(&resource_manager).await?;
    print_summary(
        config,
        &site_manager.active_address()?,
        &site_id,
        &response,
        &summary,
    )?;
    Ok(())
}

fn print_summary(
    config: &Config,
    address: &SuiAddress,
    site_id: &SiteIdentifier,
    response: &SuiTransactionBlockResponse,
    summary: &OperationsSummary,
) -> Result<()> {
    println!("{}\n", summary);

    let object_id = match site_id {
        SiteIdentifier::ExistingSite(id) => {
            println!("Updated site at object ID: {}", id);
            *id
        }
        SiteIdentifier::NewSite(name) => {
            let id = get_site_id_from_response(
                *address,
                response
                    .effects
                    .as_ref()
                    .ok_or(anyhow::anyhow!("response did not contain effects"))?,
            )?;
            println!("Created new site: {}\nNew site object ID: {}", name, id);
            id
        }
    };

    println!(
        "\nBrowse the resulting site at: https://{}.{}",
        id_to_base36(&object_id)?,
        config.portal
    );
    Ok(())
}

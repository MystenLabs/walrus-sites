// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{
    path::{Path, PathBuf},
    sync::mpsc::channel,
};

use anyhow::{anyhow, Result};
use clap::Parser;
use notify::{RecursiveMode, Watcher};
use sui_sdk::rpc_types::{
    SuiExecutionStatus,
    SuiTransactionBlockEffects,
    SuiTransactionBlockResponse,
};
use sui_types::base_types::{ObjectID, SuiAddress};

use crate::{
    display,
    preprocessor::Preprocessor,
    site::{
        manager::{SiteIdentifier, SiteManager},
        // manager::SiteManager,
        resource::{OperationsSummary, ResourceManager},
    },
    util::{get_site_id_from_response, id_to_base36, load_wallet_context},
    walrus::Walrus,
    Config,
};

#[derive(Parser, Debug, Clone)]
pub struct PublishOptions {
    /// The directory containing the site sources.
    pub directory: PathBuf,
    /// The path to the ws-resources.json file for defining HTTP resoibse headers
    /// and other utilities for your files.
    #[clap(long, default_value = "ws-resources.json")]
    ws_resources: Option<PathBuf>,
    /// The number of epochs for which to save the resources on Walrus.
    #[clap(long, default_value_t = 1)]
    pub epochs: u64,
    /// Preprocess the directory before publishing.
    /// See the `list-directory` command. Warning: Rewrites all `index.html` files.
    #[clap(long, action)]
    pub list_directory: bool,
}

pub async fn publish_site(
    publish_options: PublishOptions,
    site_name: String,
    config: &Config,
) -> Result<()> {
    edit_site(
        &publish_options.directory,
        SiteIdentifier::NewSite(site_name),
        config,
        publish_options.epochs,
        false,
        publish_options.list_directory,
        publish_options
            .ws_resources
            .expect("Please provide ws-resources.")
            .as_path(),
    )
    .await
}

pub async fn watch_edit_site(
    directory: &Path,
    site_id: SiteIdentifier,
    config: &Config,
    epochs: u64,
    force: bool,
    preprocess: bool,
    ws_resources_path: &Path,
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
                    site_id.clone(),
                    config,
                    epochs,
                    force,
                    preprocess,
                    ws_resources_path,
                )
                .await?;
            }
            Err(e) => println!("Watch error!: {}", e),
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn update_site(
    publish_options: PublishOptions,
    site_object: &ObjectID,
    config: &Config,
    watch: bool,
    force: bool,
) -> Result<()> {
    if watch {
        watch_edit_site(
            publish_options.directory.as_path(),
            SiteIdentifier::ExistingSite(*site_object),
            config,
            publish_options.epochs,
            force,
            publish_options.list_directory,
            &publish_options
                .ws_resources
                .expect("Please provide ws-resources."),
        )
        .await
    } else {
        edit_site(
            publish_options.directory.as_path(),
            SiteIdentifier::ExistingSite(*site_object),
            config,
            publish_options.epochs,
            force,
            publish_options.list_directory,
            &publish_options
                .ws_resources
                .expect("Please provide ws-resources."),
        )
        .await
    }
}

pub async fn edit_site(
    directory: &Path,
    site_id: SiteIdentifier,
    config: &Config,
    epochs: u64,
    force: bool,
    preprocess: bool,
    ws_resources_path: &Path,
) -> Result<()> {
    tracing::debug!(?site_id, ?directory, ?epochs, ?force, "editing site");

    if preprocess {
        display::action(format!("Preprocessing: {}", directory.display()));
        Preprocessor::preprocess(directory)?;
        display::done();
    }

    let wallet = load_wallet_context(&config.general.wallet)?;

    let walrus = Walrus::new(
        config.walrus_binary(),
        config.gas_budget(),
        config.general.rpc_url.clone(),
        config.general.walrus_config.clone(),
        config.general.wallet.clone(),
    );

    let mut resource_manager =
        ResourceManager::new(walrus.clone(), ws_resources_path.to_path_buf())?;
    display::action(format!(
        "Parsing the directory {} and locally computing blob IDs",
        directory.to_string_lossy()
    ));
    resource_manager.read_dir(directory)?;
    display::done();
    tracing::debug!(resources=%resource_manager.resources, "resources loaded from directory");

    let site_manager = SiteManager::new(
        config.clone(),
        walrus,
        wallet,
        site_id.clone(),
        epochs,
        force,
    )
    .await?;
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
    if let Some(SuiTransactionBlockEffects::V1(eff)) = response.effects.as_ref() {
        if let SuiExecutionStatus::Failure { error } = &eff.status {
            return Err(anyhow!(
                "error while processing the Sui transaction: {}",
                error
            ));
        }
    }

    display::header("Execution completed");
    println!("{}\n", summary);
    let object_id = match site_id {
        SiteIdentifier::ExistingSite(id) => {
            println!("Site object ID: {}", id);
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
        "Browse the resulting site at: https://{}.{}",
        id_to_base36(&object_id)?,
        config.portal
    );
    Ok(())
}

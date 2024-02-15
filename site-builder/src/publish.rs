use std::{path::Path, sync::mpsc::channel};

use anyhow::{anyhow, Result};
use notify::{RecursiveMode, Watcher};
use sui_sdk::rpc_types::{
    SuiTransactionBlockEffects,
    SuiTransactionBlockEffectsAPI,
    SuiTransactionBlockResponse,
};
use sui_types::base_types::ObjectID;

use crate::{
    site::{
        content::ContentEncoding,
        manager::SiteManager,
        resource::{Resource, ResourceManager},
    },
    util::id_to_base36,
    Config,
};

pub async fn publish_site(
    directory: &Path,
    content_encoding: &ContentEncoding,
    site_name: &str,
    config: &Config,
) -> Result<()> {
    edit_site(directory, content_encoding, site_name, &None, config).await
}

pub async fn update_site(
    directory: &Path,
    content_encoding: &ContentEncoding,
    site_object: &ObjectID,
    config: &Config,
    watch: bool,
) -> Result<()> {
    if watch {
        watch_edit_site(directory, content_encoding, "", &Some(*site_object), config).await
    } else {
        edit_site(directory, content_encoding, "", &Some(*site_object), config).await
    }
}

pub async fn edit_site(
    directory: &Path,
    content_encoding: &ContentEncoding,
    site_name: &str,
    site_object: &Option<ObjectID>,
    config: &Config,
) -> Result<()> {
    let resources = Resource::iter_dir(directory, content_encoding)?;
    let mut resource_manager = ResourceManager::default();
    for res in resources {
        resource_manager.add_resource(res);
    }
    println!("{}", resource_manager);
    let mut site_manger = SiteManager::new(*site_object, config).await?;
    let responses = site_manger
        .update_site(site_name, &mut resource_manager)
        .await?;
    print_effects(config, site_name, site_object, &responses)?;
    Ok(())
}

pub async fn watch_edit_site(
    directory: &Path,
    content_encoding: &ContentEncoding,
    site_name: &str,
    site_object: &Option<ObjectID>,
    config: &Config,
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
                edit_site(directory, content_encoding, site_name, site_object, config).await?;
            }
            Err(e) => println!("Watch error!: {}", e),
        }
    }
}

fn print_effects(
    config: &Config,
    site_name: &str,
    site_id: &Option<ObjectID>,
    responses: &[SuiTransactionBlockResponse],
) -> Result<()> {
    if responses.is_empty() {
        println!("No operation required. Site already in place.");
        return Ok(());
    }
    let effects = responses
        .iter()
        .map(|r| r.effects.clone().ok_or(anyhow!("No effects found")))
        .collect::<Result<Vec<SuiTransactionBlockEffects>>>()?;
    let (object_id, edit_string) = match site_id {
        Some(id) => (*id, format!("Blocksite updated: {}", id)),
        None => {
            let id = effects[0]
                .created()
                .iter()
                .find(|c| c.owner == config.network.address())
                .expect("Could not find the object ID for the created blocksite.")
                .reference
                .object_id;
            (id, format!("New blocksite '{}' created: {}", site_name, id))
        }
    };

    let (computation, storage, rebate, non_ref) =
        effects.iter().fold((0, 0, 0, 0), |mut total, e| {
            let summary = e.gas_cost_summary();
            total.0 += summary.computation_cost;
            total.1 += summary.storage_cost;
            total.2 += summary.storage_rebate;
            total.3 += summary.non_refundable_storage_fee;
            total
        });
    let total_cost = computation as i64 + storage as i64 - rebate as i64;

    // Print all
    println!("\n# Effects");
    println!("{}", edit_string);
    let base36 = id_to_base36(&object_id).expect("Could not convert the id to base 36.");
    println!(
        "Find it at https://{}.blocksite.net\nor http://{}.localhost:8080",
        &base36, &base36,
    );
    if let Some(explorer_url) = config.network.explorer_url(&object_id) {
        println!("(explorer url: {})\n", explorer_url);
    }

    println!("Gas cost summary (MIST):");
    println!("  - Computation: {}", computation);
    println!("  - Storage: {}", storage);
    println!("  - Storage rebate: {}", rebate);
    println!("  - Non refundable storage: {}", non_ref);
    println!(
        "For a total cost of: {} SUI{}",
        (total_cost) as f64 / 1e9,
        if total_cost < 0 {
            " (you gained SUI by deleting objects)"
        } else {
            ""
        }
    );
    Ok(())
}

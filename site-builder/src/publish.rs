use std::path::Path;

use anyhow::{anyhow, Result};
use sui_sdk::rpc_types::{
    SuiTransactionBlockEffects, SuiTransactionBlockEffectsAPI, SuiTransactionBlockResponse,
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

pub async fn publish(
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
    let mut site_manger = SiteManager::new(*site_object, config.clone()).await?;
    let responses = site_manger
        .publish_site(site_name, &mut resource_manager)
        .await?;
    print_effects(config, site_name, site_object, &responses)?;
    Ok(())
}

fn print_effects(
    config: &Config,
    site_name: &str,
    site_id: &Option<ObjectID>,
    responses: &[SuiTransactionBlockResponse],
) -> Result<()> {
    if responses.is_empty() {
        println!("No operation required. Site already published.");
        return Ok(());
    }
    let effects = responses
        .iter()
        .map(|r| r.effects.clone().ok_or(anyhow!("No effects found")))
        .collect::<Result<Vec<SuiTransactionBlockEffects>>>()?;
    let object_id = match site_id {
        Some(id) => *id,
        None => {
            effects[0]
                .created()
                .iter()
                .find(|c| c.owner == config.address)
                .expect("Could not find the object ID for the created blocksite.")
                .reference
                .object_id
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
    let total_cost = computation + storage - rebate;

    // Print all
    println!("\n# Effects");
    println!("New blocksite '{}' created: {}", site_name, object_id);
    let base36 = id_to_base36(&object_id).expect("Could not convert the id to base 36.");
    println!(
        "Find it at https://{}.blocksite.net\nor http://{}.localhost:8000\n(explorer url: {})\n",
        &base36,
        &base36,
        config.network.explorer_url(&object_id),
    );

    println!("Gas cost summary (MIST):");
    println!("  - Computation: {}", computation);
    println!("  - Storage: {}", storage);
    println!("  - Storage rebate: {}", rebate);
    println!("  - Non refundable storage: {}", non_ref);
    println!("For a total cost of: {} SUI", (total_cost) as f64 / 1e9);
    Ok(())
}

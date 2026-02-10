// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, time::Duration};

use site_builder::args::{Commands, EpochCountOrMax};

#[allow(dead_code)]
mod localnode;
use localnode::{
    args_builder::{ArgsBuilder, PublishOptionsBuilder},
    TestSetup,
};

#[allow(dead_code)]
mod helpers;
use helpers::{create_test_site, verify_resource_and_get_content};

/// Test that blob deduplication correctly keeps the blob with the highest `end_epoch`
/// when duplicate blob objects exist after expired blobs are re-stored.
#[tokio::test]
#[ignore]
async fn test_blob_dedup_keeps_highest_end_epoch() -> anyhow::Result<()> {
    const EPOCH_DURATION_SECS: u64 = 30;

    let mut cluster =
        TestSetup::start_local_test_cluster(Some(Duration::from_secs(EPOCH_DURATION_SECS))).await?;

    let temp_dir = create_test_site(1)?;
    let directory = temp_dir.path().to_path_buf();

    // Step 1: Publish for 1 epoch (blobs will expire quickly).
    let publish_epoch = cluster.current_walrus_epoch().await?;
    println!("Publishing at epoch {publish_epoch} for 1 epoch...");

    let publish_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Publish {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory.clone())
                .with_epoch_count_or_max(EpochCountOrMax::Epochs(1_u32.try_into()?))
                .build()?,
            site_name: Some("Dedup Test Site".to_string()),
        })
        .build()?;
    site_builder::run(publish_args).await?;

    let site = cluster.last_site_created().await?;
    let site_id = *site.id.object_id();
    let wallet_address = cluster.wallet_active_address()?;
    println!("Published site: {site_id}");

    // Record initial blob state.
    let initial_blobs = cluster.get_owned_blobs(wallet_address).await?;
    let end_epoch = initial_blobs
        .first()
        .expect("Should have at least one blob")
        .storage
        .end_epoch;
    println!(
        "Initial blobs: {} total, end_epoch = {end_epoch}",
        initial_blobs.len()
    );

    // Step 2: Wait for blobs to expire.
    println!("Waiting for epoch {end_epoch} (blobs expire)...");
    tokio::time::timeout(
        Duration::from_secs(EPOCH_DURATION_SECS * 3),
        cluster.wait_for_epoch(end_epoch),
    )
    .await
    .expect("timed out waiting for epoch");

    let current_epoch = cluster.current_walrus_epoch().await?;
    println!("Current epoch: {current_epoch}, blobs expired at: {end_epoch}");
    assert!(current_epoch >= end_epoch, "Blobs should be expired");

    // Step 3: Update the site for a longer duration without modifying any files.
    println!("Updating site with 10 epochs (no file changes, expired blobs re-stored)...");
    let update_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Update {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory)
                .with_epoch_count_or_max(EpochCountOrMax::Epochs(10_u32.try_into()?))
                .build()?,
            object_id: site_id,
        })
        .build()?;
    site_builder::run(update_args).await?;
    println!("Update succeeded");

    // Step 4: Verify duplicate blob objects exist.
    // The wallet now contains both old (expired) and new blob objects with the same blob_id.
    let all_blobs = cluster.get_owned_blobs(wallet_address).await?;

    let mut blob_id_groups: HashMap<_, Vec<_>> = HashMap::new();
    for blob in &all_blobs {
        blob_id_groups.entry(blob.blob_id).or_default().push(blob);
    }

    let duplicated_count = blob_id_groups.values().filter(|v| v.len() > 1).count();
    println!(
        "After update: {} unique blob_ids, {} with duplicates, {} total blob objects",
        blob_id_groups.len(),
        duplicated_count,
        all_blobs.len()
    );

    for (blob_id, blobs) in &blob_id_groups {
        let end_epochs: Vec<u32> = blobs.iter().map(|b| b.storage.end_epoch).collect();
        println!("  blob_id {blob_id}: end_epochs = {end_epochs:?}");
    }

    // We expect duplicates: the original expired blob objects and the newly re-stored ones.
    assert!(
        duplicated_count > 0,
        "Expected duplicate blob objects (expired + re-stored with same blob_id). \
         Found {} unique blob_ids across {} total objects.",
        blob_id_groups.len(),
        all_blobs.len()
    );

    // Verify the duplicated blobs have different end_epochs.
    for (blob_id, blobs) in blob_id_groups.iter().filter(|(_, v)| v.len() > 1) {
        let end_epochs: Vec<u32> = blobs.iter().map(|b| b.storage.end_epoch).collect();
        let min_epoch = end_epochs.iter().min().unwrap();
        let max_epoch = end_epochs.iter().max().unwrap();
        assert!(
            max_epoch > min_epoch,
            "Expected different end_epochs for blob_id {blob_id}, got {end_epochs:?}"
        );
    }

    // Step 5: Verify all resources are readable.
    let updated_resources = cluster.site_resources(site_id).await?;
    assert!(
        !updated_resources.is_empty(),
        "Site should still have resources after update"
    );

    for resource in &updated_resources {
        verify_resource_and_get_content(&cluster, resource).await?;
    }

    println!(
        "All {} resources verified readable — dedup correctly resolved duplicates",
        updated_resources.len()
    );

    Ok(())
}

// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Tests for epoch-related PublishOptions arguments with quilts (PublishQuilts and UpdateQuilts).
//!
//! These tests verify that the various epoch configuration options work correctly:
//! - `--epochs N`: Store blobs for N epochs
//! - `--epochs max`: Store blobs for the maximum number of epochs allowed by the system
//! - `--earliest-expiry-time`: Store until at least a specific time
//! - `--end-epoch`: Store until a specific epoch number

#![cfg(feature = "quilts-experimental")]

use std::{
    fs::File,
    io::Write,
    num::NonZeroU32,
    time::{Duration, SystemTime},
};

use site_builder::args::{Commands, EpochCountOrMax};

#[allow(dead_code)]
mod localnode;
use localnode::{
    args_builder::{ArgsBuilder, PublishOptionsBuilder},
    TestSetup,
};

#[allow(dead_code)]
mod helpers;
use helpers::{calculate_min_end_epoch_for_expiry, get_blobs_for_resources};

/// Helper to create a simple test site with a few files.
/// Adds a unique identifier to prevent blob deduplication across different test runs.
fn create_test_site(directory: &std::path::Path, num_files: usize) -> anyhow::Result<()> {
    // Use directory path hash as a unique identifier to prevent blob deduplication across tests
    use std::{
        collections::hash_map::DefaultHasher,
        hash::{Hash, Hasher},
    };

    let mut hasher = DefaultHasher::new();
    directory.hash(&mut hasher);
    let unique_id = hasher.finish();

    for i in 0..num_files {
        let file_path = directory.join(format!("file_{i}.html"));
        let mut file = File::create(file_path)?;
        writeln!(file, "<html><body>")?;
        writeln!(file, "<h1>Test File {i}</h1>")?;
        writeln!(file, "<!-- Unique test ID: {unique_id} -->")?;
        writeln!(file, "</body></html>")?;
    }

    Ok(())
}

#[tokio::test]
#[ignore]
async fn quilts_publish_with_specific_epochs() -> anyhow::Result<()> {
    const NUM_EPOCHS: u32 = 5;
    const NUM_FILES: usize = 3;

    let mut cluster = TestSetup::start_local_test_cluster().await?;
    let temp_dir = tempfile::tempdir()?;
    let directory = temp_dir.path().to_path_buf();

    println!("Creating test site with {NUM_FILES} files...");
    create_test_site(&directory, NUM_FILES)?;

    let current_epoch = cluster.get_current_epoch().await?;
    println!("Current epoch: {current_epoch}");

    println!("Publishing site with --epochs {NUM_EPOCHS}...");
    let args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::PublishQuilts {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory)
                .with_epoch_count_or_max(EpochCountOrMax::Epochs(
                    NonZeroU32::new(NUM_EPOCHS).unwrap(),
                ))
                .build()?,
            site_name: Some("Epochs Test Site".to_string()),
        })
        .build()?;

    site_builder::run(args).await?;

    // Verify the site was created
    let site = cluster.last_site_created().await?;
    let resources = cluster.site_resources(*site.id.object_id()).await?;
    assert_eq!(resources.len(), NUM_FILES);

    println!("Successfully published site with {NUM_FILES} resources using --epochs {NUM_EPOCHS}");

    // Verify the blobs have the correct end_epoch
    // Note: With quilts, multiple resources may share the same blob
    let wallet_address = cluster.wallet.inner.active_address()?;
    let blobs = get_blobs_for_resources(&cluster, wallet_address, &resources).await?;

    assert!(
        !blobs.is_empty(),
        "Expected at least one blob for the site's resources"
    );

    let expected_end_epoch = current_epoch + NUM_EPOCHS as u64;
    cluster.wait_for_user_input().await?;
    for blob in &blobs {
        assert_eq!(
            blob.storage.end_epoch as u64, expected_end_epoch,
            "Blob {} should have end_epoch {} (current {} + {} epochs), but has {}",
            blob.blob_id, expected_end_epoch, current_epoch, NUM_EPOCHS, blob.storage.end_epoch
        );
    }

    println!(
        "Verified all {} blob(s) have end_epoch = {}",
        blobs.len(),
        expected_end_epoch
    );

    Ok(())
}

#[tokio::test]
#[ignore]
async fn quilts_publish_with_epochs_max() -> anyhow::Result<()> {
    const NUM_FILES: usize = 3;

    let mut cluster = TestSetup::start_local_test_cluster().await?;
    let temp_dir = tempfile::tempdir()?;
    let directory = temp_dir.path().to_path_buf();

    println!("Creating test site with {NUM_FILES} files...");
    create_test_site(&directory, NUM_FILES)?;

    let current_epoch = cluster.get_current_epoch().await?;
    println!("Current epoch: {current_epoch}");

    println!("Publishing site with --epochs max...");
    let args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::PublishQuilts {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory)
                .with_epoch_count_or_max(EpochCountOrMax::Max)
                .build()?,
            site_name: Some("Epochs Max Test Site".to_string()),
        })
        .build()?;

    site_builder::run(args).await?;

    // Verify the site was created
    let site = cluster.last_site_created().await?;
    let resources = cluster.site_resources(*site.id.object_id()).await?;
    assert_eq!(resources.len(), NUM_FILES);

    println!("Successfully published site with {NUM_FILES} resources using --epochs max");

    // Verify the blobs were created with max epochs
    // Note: With quilts, multiple resources may share the same blob
    let wallet_address = cluster.wallet.inner.active_address()?;
    let blobs = get_blobs_for_resources(&cluster, wallet_address, &resources).await?;

    assert!(
        !blobs.is_empty(),
        "Expected at least one blob for the site's resources"
    );

    // With --epochs max, all blobs should have the same (maximum) end_epoch
    // We just verify they all have a high end_epoch value (much greater than current)
    for blob in &blobs {
        assert!(
            blob.storage.end_epoch as u64 > current_epoch + 100,
            "Blob {} should have high end_epoch with --epochs max, current {} + max, but has {}",
            blob.blob_id,
            current_epoch,
            blob.storage.end_epoch
        );
    }

    println!(
        "Verified all {} blob(s) have max end_epoch (all > {})",
        blobs.len(),
        current_epoch + 100
    );

    Ok(())
}

#[tokio::test]
#[ignore]
async fn quilts_update_with_different_epochs() -> anyhow::Result<()> {
    const INITIAL_EPOCHS: u32 = 2;
    const UPDATE_EPOCHS: u32 = 10;
    const NUM_FILES: usize = 3;

    let mut cluster = TestSetup::start_local_test_cluster().await?;
    let temp_dir = tempfile::tempdir()?;
    let directory = temp_dir.path().to_path_buf();

    println!("Creating test site with {NUM_FILES} files...");
    create_test_site(&directory, NUM_FILES)?;

    println!("Publishing site with --epochs {INITIAL_EPOCHS}...");
    let publish_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::PublishQuilts {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory.clone())
                .with_epoch_count_or_max(EpochCountOrMax::Epochs(
                    NonZeroU32::new(INITIAL_EPOCHS).unwrap(),
                ))
                .build()?,
            site_name: Some("Update Epochs Test Site".to_string()),
        })
        .build()?;

    site_builder::run(publish_args).await?;

    let site = cluster.last_site_created().await?;
    let site_id = *site.id.object_id();
    let initial_resources = cluster.site_resources(site_id).await?;
    assert_eq!(initial_resources.len(), NUM_FILES);

    // Modify one file
    println!("Modifying a file...");
    let file_to_modify = directory.join("file_0.html");
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(file_to_modify)?;
    writeln!(file, "<!-- Updated content -->")?;
    drop(file);

    println!("Updating site with --epochs {UPDATE_EPOCHS}...");
    let update_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::UpdateQuilts {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory)
                .with_epoch_count_or_max(EpochCountOrMax::Epochs(
                    NonZeroU32::new(UPDATE_EPOCHS).unwrap(),
                ))
                .build()?,
            object_id: site_id,
        })
        .build()?;

    site_builder::run(update_args).await?;

    // Verify the update succeeded
    let updated_resources = cluster.site_resources(site_id).await?;
    assert_eq!(updated_resources.len(), NUM_FILES);

    println!(
        "Successfully updated site with different epoch count (initial: {INITIAL_EPOCHS}, update: {UPDATE_EPOCHS})"
    );

    // Verify the blobs have the updated epoch (from the update command)
    let current_epoch_after_update = cluster.get_current_epoch().await?;
    let wallet_address = cluster.wallet.inner.active_address()?;
    let blobs = cluster.get_owned_blobs(wallet_address).await?;

    // We should have NUM_FILES blobs (modified file gets new blob, unmodified files keep old blobs)
    assert!(
        blobs.len() >= NUM_FILES,
        "Expected at least {} blob objects",
        NUM_FILES
    );

    // The newly created/updated blob should have end_epoch based on UPDATE_EPOCHS
    let expected_end_epoch = current_epoch_after_update + UPDATE_EPOCHS as u64;

    // At least one blob should have the new end_epoch (the updated file)
    let updated_blobs: Vec<_> = blobs
        .iter()
        .filter(|b| b.storage.end_epoch as u64 == expected_end_epoch)
        .collect();

    assert!(
        !updated_blobs.is_empty(),
        "At least one blob should have the new end_epoch {} (current {} + {} epochs)",
        expected_end_epoch,
        current_epoch_after_update,
        UPDATE_EPOCHS
    );

    println!(
        "Verified {} blob(s) have updated end_epoch = {}",
        updated_blobs.len(),
        expected_end_epoch
    );

    Ok(())
}

#[tokio::test]
#[ignore]
async fn quilts_publish_with_earliest_expiry_time() -> anyhow::Result<()> {
    const NUM_FILES: usize = 3;

    let mut cluster = TestSetup::start_local_test_cluster().await?;
    let temp_dir = tempfile::tempdir()?;
    let directory = temp_dir.path().to_path_buf();

    println!("Creating test site with {NUM_FILES} files...");
    create_test_site(&directory, NUM_FILES)?;

    // Set expiry time to 30 days from now
    let expiry_time = SystemTime::now() + Duration::from_secs(30 * 24 * 60 * 60);

    println!("Publishing site with --earliest-expiry-time...");
    let args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::PublishQuilts {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory)
                .with_earliest_expiry_time(expiry_time)
                .build()?,
            site_name: Some("Expiry Time Test Site".to_string()),
        })
        .build()?;

    site_builder::run(args).await?;

    // Verify the site was created
    let site = cluster.last_site_created().await?;
    let resources = cluster.site_resources(*site.id.object_id()).await?;
    assert_eq!(resources.len(), NUM_FILES);

    println!("Successfully published site with {NUM_FILES} resources using --earliest-expiry-time");

    // Verify the blobs have end_epoch that satisfies the expiry time requirement
    // Note: With quilts, multiple resources may share the same blob
    let wallet_address = cluster.wallet.inner.active_address()?;
    let blobs = get_blobs_for_resources(&cluster, wallet_address, &resources).await?;

    assert!(
        !blobs.is_empty(),
        "Expected at least one blob for the site's resources"
    );

    // Calculate expected minimum end_epoch based on expiry time
    let current_epoch = cluster.get_current_epoch().await?;
    let epoch_duration_ms = cluster.get_epoch_duration_ms().await?;
    let min_expected_end_epoch =
        calculate_min_end_epoch_for_expiry(expiry_time, current_epoch, epoch_duration_ms)?;

    for blob in &blobs {
        assert!(
            blob.storage.end_epoch as u64 >= min_expected_end_epoch,
            "Blob {} should have end_epoch >= {} (satisfies 30-day expiry), but has {}",
            blob.blob_id,
            min_expected_end_epoch,
            blob.storage.end_epoch
        );
    }

    println!(
        "Verified all {} blob(s) have end_epoch >= {} (satisfies 30-day expiry requirement, epoch_duration={}ms)",
        blobs.len(),
        min_expected_end_epoch,
        epoch_duration_ms
    );

    Ok(())
}

#[tokio::test]
#[ignore]
async fn quilts_update_with_earliest_expiry_time() -> anyhow::Result<()> {
    const NUM_FILES: usize = 3;

    let mut cluster = TestSetup::start_local_test_cluster().await?;
    let temp_dir = tempfile::tempdir()?;
    let directory = temp_dir.path().to_path_buf();

    println!("Creating test site with {NUM_FILES} files...");
    create_test_site(&directory, NUM_FILES)?;

    // Initial publish with 2 epochs
    println!("Publishing site with --epochs 2...");
    let publish_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::PublishQuilts {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory.clone())
                .with_epoch_count_or_max(EpochCountOrMax::Epochs(NonZeroU32::new(2).unwrap()))
                .build()?,
            site_name: Some("Update Expiry Time Test Site".to_string()),
        })
        .build()?;

    site_builder::run(publish_args).await?;

    let site = cluster.last_site_created().await?;
    let site_id = *site.id.object_id();

    // Modify one file
    println!("Modifying a file...");
    let file_to_modify = directory.join("file_1.html");
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(file_to_modify)?;
    writeln!(file, "<!-- Updated with expiry time -->")?;
    drop(file);

    // Update with earliest_expiry_time
    let expiry_time = SystemTime::now() + Duration::from_secs(60 * 24 * 60 * 60);

    println!("Updating site with --earliest-expiry-time...");
    let update_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::UpdateQuilts {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory)
                .with_earliest_expiry_time(expiry_time)
                .build()?,
            object_id: site_id,
        })
        .build()?;

    site_builder::run(update_args).await?;

    // Verify the update succeeded
    let updated_resources = cluster.site_resources(site_id).await?;
    assert_eq!(updated_resources.len(), NUM_FILES);

    println!("Successfully updated site using --earliest-expiry-time");

    // Verify updated blobs have appropriate end_epochs
    let wallet_address = cluster.wallet.inner.active_address()?;
    let blobs = cluster.get_owned_blobs(wallet_address).await?;

    let current_epoch = cluster.get_current_epoch().await?;

    // At least some blobs should have high end_epoch (from the update with 60-day expiry)
    let high_epoch_blobs: Vec<_> = blobs
        .iter()
        .filter(|b| b.storage.end_epoch as u64 > current_epoch + 10)
        .collect();

    assert!(
        !high_epoch_blobs.is_empty(),
        "At least one blob should have high end_epoch from the update"
    );

    println!(
        "Verified {} blob(s) have high end_epoch from --earliest-expiry-time update",
        high_epoch_blobs.len()
    );

    Ok(())
}

#[tokio::test]
#[ignore]
async fn quilts_publish_with_end_epoch() -> anyhow::Result<()> {
    const NUM_FILES: usize = 3;
    const END_EPOCH: u32 = 100;

    let mut cluster = TestSetup::start_local_test_cluster().await?;
    let temp_dir = tempfile::tempdir()?;
    let directory = temp_dir.path().to_path_buf();

    println!("Creating test site with {NUM_FILES} files...");
    create_test_site(&directory, NUM_FILES)?;

    let current_epoch = cluster.get_current_epoch().await?;
    println!("Current epoch: {current_epoch}");

    println!("Publishing site with --end-epoch {END_EPOCH}...");
    let args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::PublishQuilts {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory)
                .with_end_epoch(NonZeroU32::new(END_EPOCH).unwrap())
                .build()?,
            site_name: Some("End Epoch Test Site".to_string()),
        })
        .build()?;

    site_builder::run(args).await?;

    // Verify the site was created
    let site = cluster.last_site_created().await?;
    let resources = cluster.site_resources(*site.id.object_id()).await?;
    assert_eq!(resources.len(), NUM_FILES);

    println!(
        "Successfully published site with {NUM_FILES} resources using --end-epoch {END_EPOCH}"
    );

    // Verify the blobs have the specified end_epoch
    // Note: With quilts, multiple resources may share the same blob
    let wallet_address = cluster.wallet.inner.active_address()?;
    let blobs = get_blobs_for_resources(&cluster, wallet_address, &resources).await?;

    assert!(
        !blobs.is_empty(),
        "Expected at least one blob for the site's resources"
    );

    // All blobs should have the correct end_epoch
    for blob in &blobs {
        assert_eq!(
            blob.storage.end_epoch, END_EPOCH,
            "Blob {} should have end_epoch {}, but has {}",
            blob.blob_id, END_EPOCH, blob.storage.end_epoch
        );
    }

    println!(
        "Verified all {} blob(s) have end_epoch = {}",
        blobs.len(),
        END_EPOCH
    );

    Ok(())
}

#[tokio::test]
#[ignore]
async fn quilts_update_with_end_epoch() -> anyhow::Result<()> {
    const NUM_FILES: usize = 3;
    const INITIAL_END_EPOCH: u32 = 50;
    const UPDATE_END_EPOCH: u32 = 150;

    let mut cluster = TestSetup::start_local_test_cluster().await?;
    let temp_dir = tempfile::tempdir()?;
    let directory = temp_dir.path().to_path_buf();

    println!("Creating test site with {NUM_FILES} files...");
    create_test_site(&directory, NUM_FILES)?;

    println!("Publishing site with --end-epoch {INITIAL_END_EPOCH}...");
    let publish_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::PublishQuilts {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory.clone())
                .with_end_epoch(NonZeroU32::new(INITIAL_END_EPOCH).unwrap())
                .build()?,
            site_name: Some("Update End Epoch Test Site".to_string()),
        })
        .build()?;

    site_builder::run(publish_args).await?;

    let site = cluster.last_site_created().await?;
    let site_id = *site.id.object_id();
    let initial_resources = cluster.site_resources(site_id).await?;
    assert_eq!(initial_resources.len(), NUM_FILES);

    // Modify one file
    println!("Modifying a file...");
    let file_to_modify = directory.join("file_2.html");
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(file_to_modify)?;
    writeln!(file, "<!-- Updated with new end epoch -->")?;
    drop(file);

    println!("Updating site with --end-epoch {UPDATE_END_EPOCH}...");
    let update_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::UpdateQuilts {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory)
                .with_end_epoch(NonZeroU32::new(UPDATE_END_EPOCH).unwrap())
                .build()?,
            object_id: site_id,
        })
        .build()?;

    site_builder::run(update_args).await?;

    // Verify the update succeeded
    let updated_resources = cluster.site_resources(site_id).await?;
    assert_eq!(updated_resources.len(), NUM_FILES);

    println!(
        "Successfully updated site with different end epoch (initial: {INITIAL_END_EPOCH}, update: {UPDATE_END_EPOCH})"
    );

    // Verify blobs have the updated end_epoch
    let wallet_address = cluster.wallet.inner.active_address()?;
    let blobs = cluster.get_owned_blobs(wallet_address).await?;

    // We should have at least NUM_FILES blobs (modified file gets new blob with UPDATE_END_EPOCH)
    assert!(
        blobs.len() >= NUM_FILES,
        "Expected at least {} blob objects",
        NUM_FILES
    );

    // At least one blob should have the new UPDATE_END_EPOCH (the updated file)
    let updated_blobs: Vec<_> = blobs
        .iter()
        .filter(|b| b.storage.end_epoch == UPDATE_END_EPOCH)
        .collect();

    assert!(
        !updated_blobs.is_empty(),
        "At least one blob should have the updated end_epoch {}",
        UPDATE_END_EPOCH
    );

    println!(
        "Verified {} blob(s) have updated end_epoch = {}",
        updated_blobs.len(),
        UPDATE_END_EPOCH
    );

    Ok(())
}

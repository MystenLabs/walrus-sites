// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Tests for epoch-related PublishOptions arguments with quilts (PublishQuilts and UpdateQuilts).
//!
//! These tests verify that the various epoch configuration options work correctly:
//! - `--epochs N`: Store blobs for N epochs
//! - `--epochs max`: Store blobs for the maximum number of epochs allowed by the system
//! - `--earliest-expiry-time`: Store until at least a specific time
//! - `--end-epoch`: Store until a specific epoch number

use std::{io::Write, num::NonZeroU32, time::SystemTime};

use site_builder::args::{Commands, EpochArg, EpochCountOrMax};

#[allow(dead_code)]
mod localnode;
use localnode::{
    args_builder::{ArgsBuilder, PublishOptionsBuilder},
    TestSetup,
};

#[allow(dead_code)]
mod helpers;
use helpers::{calculate_min_end_epoch_for_expiry, create_test_site, get_blobs_for_resources};

/// Number of files to create in each test site
const NUM_FILES: usize = 3;

/// Maximum number of epochs ahead that the system allows (from Walrus Move contract)
const MAX_EPOCHS_AHEAD: u32 = 53;

/// Sets up a test cluster and publishes a site with the given number of files and epoch configuration.
/// Returns the cluster, temp directory (must be kept alive), directory path, and site object ID.
async fn setup_test_cluster_and_site(
    num_files: usize,
    epoch_arg: EpochArg,
) -> anyhow::Result<(
    TestSetup,
    tempfile::TempDir,
    std::path::PathBuf,
    sui_types::base_types::ObjectID,
)> {
    let cluster = TestSetup::start_local_test_cluster().await?;

    println!("Creating test site with {num_files} files...");
    let temp_dir = create_test_site(num_files)?;
    let directory = temp_dir.path().to_path_buf();

    println!("Publishing site...");
    let args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Publish {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory.clone())
                .with_epoch_arg(epoch_arg)
                .build()?,
            site_name: Some("Test Site".to_string()),
        })
        .build()?;

    site_builder::run(args).await?;

    let site = cluster.last_site_created().await?;
    let site_id = *site.id.object_id();

    Ok((cluster, temp_dir, directory, site_id))
}

#[tokio::test]
#[ignore]
async fn quilts_publish_with_specific_epochs() -> anyhow::Result<()> {
    const NUM_EPOCHS: u32 = 5;

    let mut cluster = TestSetup::start_local_test_cluster().await?;

    println!("Creating test site with {NUM_FILES} files...");
    let temp_dir = create_test_site(NUM_FILES)?;
    let directory = temp_dir.path().to_path_buf();

    let current_epoch = cluster.current_walrus_epoch().await?;
    println!("Current epoch: {current_epoch}");

    println!("Publishing site with --epochs {NUM_EPOCHS}...");
    let args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Publish {
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

    // When requesting N epochs, end_epoch = start_epoch + N = current_epoch  + N
    let expected_end_epoch = current_epoch + NUM_EPOCHS;
    for blob in &blobs {
        assert_eq!(
            blob.storage.end_epoch, expected_end_epoch,
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
    let mut cluster = TestSetup::start_local_test_cluster().await?;

    println!("Creating test site with {NUM_FILES} files...");
    let temp_dir = create_test_site(NUM_FILES)?;
    let directory = temp_dir.path().to_path_buf();

    let current_epoch = cluster.current_walrus_epoch().await?;
    println!("Current epoch: {current_epoch}");

    println!("Publishing site with --epochs max...");
    let args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Publish {
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
    // The system limit is 53 epochs ahead, so end_epoch = current_epoch + MAX_EPOCHS_AHEAD
    let expected_max_end_epoch = current_epoch + MAX_EPOCHS_AHEAD;
    for blob in &blobs {
        assert_eq!(
            blob.storage.end_epoch, expected_max_end_epoch,
            "Blob {} should have max end_epoch {} (current {} + 53) with --epochs max, but has {}",
            blob.blob_id, expected_max_end_epoch, current_epoch, blob.storage.end_epoch
        );
    }

    println!(
        "Verified all {} blob(s) have max end_epoch = {} (current {} + {})",
        blobs.len(),
        expected_max_end_epoch,
        current_epoch,
        MAX_EPOCHS_AHEAD
    );

    Ok(())
}

#[tokio::test]
#[ignore]
async fn quilts_update_with_different_epochs() -> anyhow::Result<()> {
    const INITIAL_EPOCHS: u32 = 2;
    const UPDATE_EPOCHS: u32 = 10;

    let (mut cluster, _temp_dir, directory, site_id) = setup_test_cluster_and_site(
        NUM_FILES,
        EpochArg {
            epochs: Some(EpochCountOrMax::Epochs(
                NonZeroU32::new(INITIAL_EPOCHS).unwrap(),
            )),
            ..Default::default()
        },
    )
    .await?;

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
        .with_command(Commands::Update {
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
    let current_epoch_after_update = cluster.current_walrus_epoch().await?;
    let wallet_address = cluster.wallet.inner.active_address()?;
    let blobs = cluster.get_owned_blobs(wallet_address).await?;

    // We should have NUM_FILES blobs (modified file gets new blob, unmodified files keep old blobs)
    assert!(
        !blobs.is_empty(),
        "Should have at least 1 Blob (Quilt) under our address"
    );

    // The newly created/updated blob should have end_epoch based on UPDATE_EPOCHS
    // When requesting N epochs, end_epoch = start_epoch + N = current_epoch + N
    let expected_end_epoch = current_epoch_after_update + UPDATE_EPOCHS;

    // At least one blob should have the new end_epoch (the updated file)
    let updated_blobs: Vec<_> = blobs
        .iter()
        .filter(|b| b.storage.end_epoch >= expected_end_epoch)
        .collect();

    assert!(
        !updated_blobs.is_empty(),
        "At least one blob should have the new end_epoch {expected_end_epoch} >= (current {current_epoch_after_update} + {UPDATE_EPOCHS} epochs)",
    );

    println!(
        "Verified {} blob(s) have updated end_epoch >= {}",
        updated_blobs.len(),
        expected_end_epoch
    );

    Ok(())
}

#[tokio::test]
#[ignore]
async fn quilts_publish_with_earliest_expiry_time() -> anyhow::Result<()> {
    let mut cluster = TestSetup::start_local_test_cluster().await?;

    println!("Creating test site with {NUM_FILES} files...");
    let temp_dir = create_test_site(NUM_FILES)?;
    let directory = temp_dir.path().to_path_buf();

    let epoch_duration_ms = cluster.epoch_duration_ms().await?;
    let epoch_start = cluster.epoch_start_timestamp().await?;

    // Set expiry time to 10 epochs from now using Walrus epoch time
    let expiry_time = SystemTime::from(
        epoch_start
            + chrono::Duration::milliseconds(
                (epoch_duration_ms * 10 + epoch_duration_ms / 2) as i64,
            ),
    );

    println!("Publishing site with --earliest-expiry-time...");
    let args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Publish {
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
    let current_epoch = cluster.current_walrus_epoch().await?;
    let epoch_duration_ms = cluster.epoch_duration_ms().await?;
    println!("epoch_duration_ms: {epoch_duration_ms}");
    let min_expected_end_epoch =
        calculate_min_end_epoch_for_expiry(expiry_time, current_epoch, epoch_duration_ms)?;

    for blob in &blobs {
        assert!(
            blob.storage.end_epoch >= min_expected_end_epoch,
            "Blob {} should have end_epoch >= {} (satisfies 10+ epochs expiry), but has {}",
            blob.blob_id,
            min_expected_end_epoch,
            blob.storage.end_epoch
        );
    }

    println!(
        "Verified all {} blob(s) have end_epoch >= {} (satisfies 10+ epochs expiry requirement, epoch_duration={}ms)",
        blobs.len(),
        min_expected_end_epoch,
        epoch_duration_ms
    );

    Ok(())
}

#[tokio::test]
#[ignore]
async fn quilts_update_with_earliest_expiry_time() -> anyhow::Result<()> {
    let (mut cluster, _temp_dir, directory, site_id) = setup_test_cluster_and_site(
        NUM_FILES,
        EpochArg {
            epochs: Some(EpochCountOrMax::Epochs(NonZeroU32::new(2).unwrap())),
            ..Default::default()
        },
    )
    .await?;

    // Modify one file
    println!("Modifying a file...");
    let file_to_modify = directory.join("file_1.html");
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(file_to_modify)?;
    writeln!(file, "<!-- Updated with expiry time -->")?;
    drop(file);

    // Update with earliest_expiry_time (20 epochs to stay within 53 epoch limit)
    let epoch_duration_ms = cluster.epoch_duration_ms().await?;
    let epoch_start = cluster.epoch_start_timestamp().await?;
    let expiry_time = SystemTime::from(
        epoch_start + chrono::Duration::milliseconds((epoch_duration_ms * 20) as i64),
    );

    println!("Updating site with --earliest-expiry-time...");
    let update_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Update {
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

    let current_epoch = cluster.current_walrus_epoch().await?;

    // At least some blobs should have high end_epoch (from the update with 60-day expiry)
    let high_epoch_blobs: Vec<_> = blobs
        .iter()
        .filter(|b| b.storage.end_epoch > current_epoch + 10)
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
    let mut cluster = TestSetup::start_local_test_cluster().await?;
    let current_epoch = cluster.current_walrus_epoch().await?;
    // Use an end_epoch that's realistic (within MAX_EPOCHS_AHEAD)
    let end_epoch = current_epoch + 50;

    println!("Creating test site with {NUM_FILES} files...");
    let temp_dir = create_test_site(NUM_FILES)?;
    let directory = temp_dir.path().to_path_buf();

    println!("Current epoch: {current_epoch}, using end_epoch: {end_epoch}");

    println!("Publishing site with --end-epoch {end_epoch}...");
    let args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Publish {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory)
                .with_end_epoch(NonZeroU32::new(end_epoch).unwrap())
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
        "Successfully published site with {NUM_FILES} resources using --end-epoch {end_epoch}"
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
            blob.storage.end_epoch, end_epoch,
            "Blob {} should have end_epoch {}, but has {}",
            blob.blob_id, end_epoch, blob.storage.end_epoch
        );
    }

    println!(
        "Verified all {} blob(s) have end_epoch = {}",
        blobs.len(),
        end_epoch
    );

    Ok(())
}

#[tokio::test]
#[ignore]
async fn quilts_update_with_end_epoch() -> anyhow::Result<()> {
    const INITIAL_END_EPOCH: u32 = 20;
    const UPDATE_END_EPOCH: u32 = 50;

    let (mut cluster, _temp_dir, directory, site_id) = setup_test_cluster_and_site(
        NUM_FILES,
        EpochArg {
            end_epoch: Some(NonZeroU32::new(INITIAL_END_EPOCH).unwrap()),
            ..Default::default()
        },
    )
    .await?;

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
        .with_command(Commands::Update {
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
    assert!(!blobs.is_empty(), "Expected at least one blob object");

    // At least one blob should have the new update_end_epoch (the updated file)
    let updated_blobs: Vec<_> = blobs
        .iter()
        .filter(|b| b.storage.end_epoch == UPDATE_END_EPOCH)
        .collect();

    assert!(
        !updated_blobs.is_empty(),
        "At least one blob should have the updated end_epoch {UPDATE_END_EPOCH}",
    );

    println!(
        "Verified {} blob(s) have updated end_epoch = {}",
        updated_blobs.len(),
        UPDATE_END_EPOCH
    );

    Ok(())
}

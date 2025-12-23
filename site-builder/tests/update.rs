// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::PathBuf,
};

use site_builder::{
    args::{Commands, EpochCountOrMax},
    site_config::WSResources,
};

#[allow(dead_code)]
mod localnode;
use localnode::{
    args_builder::{ArgsBuilder, PublishOptionsBuilder},
    TestSetup,
};

#[allow(dead_code)]
mod helpers;
use helpers::{create_test_site, verify_resource_and_get_content};

#[tokio::test]
#[ignore]
async fn update_snake() -> anyhow::Result<()> {
    let mut cluster = TestSetup::start_local_test_cluster().await?;
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("examples")
        .join("snake");

    // Copy snake and reset ws-resources object_id
    let temp_dir = tempfile::tempdir()?;
    helpers::copy_dir(directory.as_path(), temp_dir.path())?;
    let directory = temp_dir.path().to_path_buf();
    let ws_resources_path = directory.join("ws-resources.json");
    let mut ws_resources_init: WSResources =
        serde_json::from_reader(File::open(ws_resources_path.as_path())?)?;
    ws_resources_init.object_id = None;
    serde_json::to_writer_pretty(
        File::create(ws_resources_path.as_path())?,
        &ws_resources_init,
    )?;

    let publish_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Publish {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory.clone())
                .with_epoch_count_or_max(EpochCountOrMax::Epochs(1_u32.try_into().unwrap()))
                .build()?,
            site_name: None,
        })
        .build()?;
    site_builder::run(publish_args).await?;

    // Store original index.html line count before update
    let original_index_content = fs::read_to_string(temp_dir.path().join("index.html"))?;
    let original_line_count = original_index_content.lines().count();

    // Update a resource
    let index_html_path = temp_dir.path().join("index.html");
    {
        let mut index_html = OpenOptions::new()
            .append(true) // don't truncate, add to the end
            .open(index_html_path)?;
        writeln!(&mut index_html, "<!-- Updated by test -->")?;
    } // File is automatically flushed and closed when it goes out of scope
    let ws_resources_updated: WSResources =
        serde_json::from_reader(File::open(ws_resources_path.as_path())?)?;

    let site_id = *cluster.last_site_created().await?.id.object_id();
    assert_eq!(ws_resources_updated.object_id.unwrap(), site_id);

    let update_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Update {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory)
                .with_epoch_count_or_max(EpochCountOrMax::Max)
                .build()?,
            object_id: site_id,
        })
        .build()?;
    site_builder::run(update_args).await?;

    // TODO: Update only stores new blobs
    // Verify that the user now has 2 blob objects:
    // - One from the old site (unchanged resources)
    // - One with only the updated index.html
    let wallet_address = cluster.wallet_active_address()?;
    let owned_blobs = cluster.get_owned_blobs(wallet_address).await?;
    assert_eq!(
        owned_blobs.len(),
        2,
        "After update, user should have exactly 2 blob objects: one from the old site and one with the updated index.html"
    );

    // Verify the update worked
    let updated_site = cluster.last_site_created().await?;
    let updated_resources = cluster.site_resources(*updated_site.id.object_id()).await?;

    // The site should still have the same object ID
    assert_eq!(site_id, *updated_site.id.object_id());

    // Verify that all resources have valid hashes
    for resource in updated_resources {
        let data = verify_resource_and_get_content(&cluster, &resource).await?;

        // For index.html, verify it has exactly one more line than the original
        if resource.path == "/index.html" {
            let content = String::from_utf8_lossy(&data);
            let updated_line_count = content.lines().count();
            assert_eq!(updated_line_count, original_line_count + 1,
                "index.html should have exactly one more line after update. Original: {original_line_count}, Updated: {updated_line_count}",
                );
        }
    }

    println!("quilts_update_snake completed successfully");

    Ok(())
}

#[tokio::test]
#[ignore]
async fn quilts_update_half_files() -> anyhow::Result<()> {
    const N_FILES_IN_SITE: usize = 100;

    let cluster = TestSetup::start_local_test_cluster().await?;

    // Create a temporary directory for our test site
    println!("Creating {N_FILES_IN_SITE} files for the test site...");
    let temp_dir = create_test_site(N_FILES_IN_SITE)?;
    let test_site_dir = temp_dir.path().to_owned();

    // Step 2: Publish the initial site
    println!("Publishing initial site with {N_FILES_IN_SITE} files...");
    let publish_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Publish {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(test_site_dir.clone())
                .with_epoch_count_or_max(EpochCountOrMax::Max)
                .build()?,
            site_name: Some("Test Multi-File Site".to_string()),
        })
        .with_gas_budget(10_000_000_000) // Higher gas budget for many files
        .build()?;

    site_builder::run(publish_args).await?;

    // Get the site object ID from the published site
    let site = cluster.last_site_created().await?;
    let site_object_id = *site.id.object_id();

    println!("Published site with object ID: {site_object_id}");

    // Verify initial publish worked correctly
    let initial_resources = cluster.site_resources(site_object_id).await?;
    assert_eq!(initial_resources.len(), N_FILES_IN_SITE);
    println!(
        "Verified {} resources in initial site",
        initial_resources.len()
    );

    // Step 3: Modify half of the files (only odd numbered files)
    println!("Modifying half of the {N_FILES_IN_SITE} files for update...");
    for i in 0..N_FILES_IN_SITE {
        if i % 2 == 0 {
            continue;
        } // Skip even numbered files
        let file_path = test_site_dir.join(format!("file_{i}.html"));
        let mut file = OpenOptions::new().append(true).open(&file_path)?;
        writeln!(file, "<!-- UPDATED Page {i} -->")?;
    }

    // Step 4: Update the site using the Update command
    println!("Updating site with modified files...");
    let update_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Update {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(test_site_dir)
                .with_epoch_count_or_max(EpochCountOrMax::Max)
                .build()?,
            object_id: site_object_id,
        })
        .with_gas_budget(10_000_000_000) // Higher gas budget for many files
        .build()?;

    site_builder::run(update_args).await?;

    println!("Successfully updated site");

    // Step 5: Verify the update worked
    let updated_site = cluster.last_site_created().await?;
    let updated_resources = cluster.site_resources(*updated_site.id.object_id()).await?;

    // The site should still have the same object ID
    assert_eq!(site_object_id, *updated_site.id.object_id());

    // Should still have the same number of resources
    assert_eq!(updated_resources.len(), N_FILES_IN_SITE);

    // Verify that all resources have valid hashes (indicating they were processed)
    println!("Verifying {} updated resources...", updated_resources.len());
    for resource in updated_resources.iter() {
        let data = verify_resource_and_get_content(&cluster, resource).await?;

        // Extract file number from path (e.g., "/file_42.html" -> 42)
        let file_number = resource
            .path
            .strip_prefix('/')
            .and_then(|p| p.strip_suffix(".html"))
            .and_then(|p| p.strip_prefix("file_"))
            .and_then(|p| p.parse::<usize>().ok())
            .unwrap_or_else(|| panic!("Could not parse file number from path: {}", resource.path));

        // Verify the content - only odd numbered files should contain "UPDATED"
        let content = String::from_utf8_lossy(&data);
        if file_number % 2 == 1 {
            assert!(
                content.contains("UPDATED"),
                "Resource {} (file {file_number}) should contain update marker",
                resource.path,
            );
        } else {
            assert!(
                !content.contains("UPDATED"),
                "Resource {} (file {file_number}) should NOT contain update marker",
                resource.path,
            );
        }
    }

    println!("Update test with {N_FILES_IN_SITE} files completed successfully!");

    Ok(())
}

/// Tests that quilts are automatically extended during updates and verifies quilt lifetime behavior.
///
/// This test verifies the following behavior:
/// 1. **Initial publish**: Creates n_slots_in_quilts + 1 files and publishes with quilts
///    - First n_slots_in_quilts files end up in one quilt (main quilt)
///    - The extra file ends up in a separate single-file quilt
///
/// 2. **First update (longer epochs)**: Modifies the single file and updates with more epochs
///    - Both quilts (main and single-file) are extended to the new epoch count
///    - Main quilt blob_id remains unchanged
///
/// 3. **Second update (shorter epochs)**: Modifies the single file again with fewer epochs
///    - Main quilt keeps its blob_id and end_epoch from the first update (not extended down)
///    - Single file gets a new blob with the shorter end_epoch
///
/// This demonstrates that:
/// - Unchanged quilts are automatically extended when update epochs are longer
/// - Unchanged quilts are NOT re-extended when update epochs are shorter
/// - Modified files get new blobs with the current update epoch settings
#[tokio::test]
#[ignore]
async fn quilts_update_check_quilt_lifetime() -> anyhow::Result<()> {
    const PUBLISH_EPOCHS: u32 = 5;
    const UPDATE_EPOCHS: u32 = 50;
    const UPDATE_EPOCHS_SHORTER: u32 = 20;

    let mut cluster = TestSetup::start_local_test_cluster().await?;

    // Calculate n_slots_in_quilts based on the cluster's n_shards
    let n_shards = cluster.cluster_state.walrus_cluster.n_shards;
    let n_slots_in_quilts =
        u16::from(walrus_core::encoding::source_symbols_for_n_shards(n_shards).1) as usize - 1;

    let n_files = n_slots_in_quilts + 1;
    // Create a temporary directory for our test site
    println!("Creating test site with {n_files} files (n_slots_in_quilts + 1)...");
    let temp_dir = create_test_site(n_files)?;
    let test_site_dir = temp_dir.path().to_owned();

    // Publish the initial site with publish-quilts
    println!("Publishing initial site with publish-quilts for {PUBLISH_EPOCHS} epochs...");
    let publish_epoch = cluster.current_walrus_epoch().await?;
    let publish_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Publish {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(test_site_dir.clone())
                .with_epoch_count_or_max(EpochCountOrMax::Epochs(PUBLISH_EPOCHS.try_into()?))
                .build()?,
            site_name: Some("Test Directory Quilt Site".to_string()),
        })
        .with_gas_budget(10_000_000_000)
        .build()?;

    site_builder::run(publish_args).await?;

    // Get the site object ID
    let site = cluster.last_site_created().await?;
    let site_object_id = *site.id.object_id();

    println!("Published site with object ID: {site_object_id} for {PUBLISH_EPOCHS} epochs.");

    let wallet_address = cluster.wallet_active_address()?;
    let owned_blobs = cluster.get_owned_blobs(wallet_address).await?;

    // Calculate expected end_epoch: current_epoch + PUBLISH_EPOCHS
    let expected_end_epoch = publish_epoch + PUBLISH_EPOCHS;

    for blob in &owned_blobs {
        println!(
            "Blob {} - end_epoch: {} (expected: {})",
            blob.blob_id, blob.storage.end_epoch, expected_end_epoch
        );
        assert_eq!(
            blob.storage.end_epoch, expected_end_epoch,
            "Blob {} should have end_epoch {} but has {}",
            blob.blob_id, expected_end_epoch, blob.storage.end_epoch
        );
    }

    // Verify initial publish worked correctly
    let initial_resources = cluster.site_resources(site_object_id).await?;
    assert_eq!(initial_resources.len(), n_files);

    // Create a map of blob_id -> vec<resource_path> to group resources by their quilt
    let mut blob_id_to_paths = std::collections::HashMap::new();
    for resource in &initial_resources {
        blob_id_to_paths
            .entry(resource.blob_id)
            .or_insert_with(Vec::new)
            .push(resource.path.clone());
    }

    let items: Vec<_> = blob_id_to_paths.iter().collect();
    let [(blob_id_first, paths_first), (blob_id_second, paths_second)] = items[..] else {
        panic!("Expected exactly 2 blobs");
    };

    // Find which blob_id has n_slots_in_quilts files and which has 1 file
    let (main_quilt_blob_id, single_file_blob_id, single_file) = {
        if paths_first.len() == n_slots_in_quilts {
            (*blob_id_first, *blob_id_second, paths_second[0].as_str())
        } else {
            (*blob_id_second, *blob_id_first, paths_first[0].as_str())
        }
    };

    println!("\n✓ Main quilt (blob {main_quilt_blob_id}): {n_slots_in_quilts} files",);
    println!("✓ Single file quilt (blob {single_file_blob_id}): 1 file ({single_file})",);

    // Modify the single file
    println!("\nModifying single file: {single_file}");
    let single_file_path = test_site_dir.join(single_file.trim_start_matches('/'));
    {
        let mut file = OpenOptions::new().append(true).open(&single_file_path)?;
        writeln!(file, "<!-- Modified by test -->")?;
        file.flush()?;
    }

    // First update: extend all blobs to UPDATE_EPOCHS
    println!("\n=== First Update: {UPDATE_EPOCHS} epochs ===");
    let update_1_epoch = cluster.current_walrus_epoch().await?;

    let update_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Update {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(test_site_dir.clone())
                .with_epoch_count_or_max(EpochCountOrMax::Epochs(UPDATE_EPOCHS.try_into()?))
                .build()?,
            object_id: site_object_id,
        })
        .with_gas_budget(10_000_000_000)
        .build()?;

    site_builder::run(update_args).await?;

    println!("Successfully ran first update for {UPDATE_EPOCHS} epochs.");

    // Verify all blobs have been extended
    let owned_blobs = cluster.get_owned_blobs(wallet_address).await?;
    let expected_end_epoch_update_1 = update_1_epoch + UPDATE_EPOCHS;

    println!("\nVerifying blob extensions after first update...");
    println!("Found {} owned blobs", owned_blobs.len());

    assert_eq!(owned_blobs.len(), 2, "Should have exactly 2 blobs");

    let blobs_after_update_1: Vec<_> = owned_blobs.iter().collect();
    let [blob_first, blob_second] = blobs_after_update_1[..] else {
        panic!("Expected exactly 2 blobs");
    };

    let (main_blob_after_update_1, single_blob_after_update_1) = {
        if blob_first.blob_id.0 == main_quilt_blob_id.0 {
            (blob_first, blob_second)
        } else {
            (blob_second, blob_first)
        }
    };

    // Verify both blobs have been extended to the same end_epoch
    assert_eq!(
        main_blob_after_update_1.storage.end_epoch, expected_end_epoch_update_1,
        "Main quilt blob should have end_epoch {expected_end_epoch_update_1}",
    );
    assert_eq!(
        single_blob_after_update_1.storage.end_epoch, expected_end_epoch_update_1,
        "Single file blob should have end_epoch {expected_end_epoch_update_1}",
    );

    println!(
        "✓ Main quilt blob {} - end_epoch: {}",
        main_quilt_blob_id, main_blob_after_update_1.storage.end_epoch
    );
    println!(
        "✓ Single file blob {} - end_epoch: {}",
        single_blob_after_update_1.blob_id, single_blob_after_update_1.storage.end_epoch
    );
    println!(
        "✓ All {} blobs extended to epoch {expected_end_epoch_update_1}",
        owned_blobs.len()
    );

    // Second update: modify single file again with shorter epochs
    println!("\n=== Second Update: {UPDATE_EPOCHS_SHORTER} epochs (shorter) ===");
    println!("Modifying single file again: {single_file}");
    {
        let mut file = OpenOptions::new().append(true).open(&single_file_path)?;
        writeln!(file, "<!-- Modified again with shorter epochs -->")?;
        file.flush()?;
    }

    let update_2_epoch = cluster.current_walrus_epoch().await?;

    let update_args_2 = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Update {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(test_site_dir)
                .with_epoch_count_or_max(EpochCountOrMax::Epochs(UPDATE_EPOCHS_SHORTER.try_into()?))
                .build()?,
            object_id: site_object_id,
        })
        .with_gas_budget(10_000_000_000)
        .build()?;

    site_builder::run(update_args_2).await?;

    println!("Successfully ran second update with {UPDATE_EPOCHS_SHORTER} epochs");

    // Get updated resources to find the new single file blob_id
    let resources_after_update_2 = cluster.site_resources(site_object_id).await?;
    let single_file_resource = resources_after_update_2
        .iter()
        .find(|r| r.path == single_file)
        .expect("Single file resource should exist");
    let new_single_file_blob_id = single_file_resource.blob_id;

    // Verify final blob state
    let owned_blobs_final = cluster.get_owned_blobs(wallet_address).await?;

    println!("\nVerifying blob end_epochs after second update...");
    println!("Found {} owned blobs", owned_blobs_final.len());

    // Expected: 2 blobs (main quilt + new single file blob)
    assert_eq!(owned_blobs_final.len(), 2, "Should have exactly 2 blobs");

    let blobs_final: Vec<_> = owned_blobs_final.iter().collect();
    let [blob_first, blob_second] = blobs_final[..] else {
        panic!("Expected exactly 2 blobs");
    };

    let (main_blob_final, single_file_blob_final) = {
        if blob_first.blob_id.0 == main_quilt_blob_id.0 {
            (blob_first, blob_second)
        } else {
            (blob_second, blob_first)
        }
    };

    assert_eq!(
        main_blob_final.storage.end_epoch, expected_end_epoch_update_1,
        "Main quilt should keep end_epoch from first update"
    );
    println!(
        "✓ Main quilt blob {} - end_epoch: {} (unchanged from first update)",
        main_blob_final.blob_id, main_blob_final.storage.end_epoch
    );

    // Verify single file blob: new blob_id with shorter end_epoch
    assert_eq!(
        single_file_blob_final.blob_id.0, new_single_file_blob_id.0,
        "Single file blob should have new blob_id"
    );
    let expected_end_epoch_update_2 = update_2_epoch + UPDATE_EPOCHS_SHORTER;
    assert_eq!(
        single_file_blob_final.storage.end_epoch, expected_end_epoch_update_2,
        "Single file blob should have end_epoch from second update"
    );
    println!(
        "✓ Single file blob {} - end_epoch: {} (new from second update)",
        single_file_blob_final.blob_id, single_file_blob_final.storage.end_epoch
    );

    Ok(())
}

#[tokio::test]
#[ignore]
async fn update_with_file_rename() -> anyhow::Result<()> {
    const N_FILES: usize = 10;

    let cluster = TestSetup::start_local_test_cluster().await?;

    // Create a temporary directory for our test site
    let temp_dir = tempfile::tempdir()?;
    let test_site_dir = temp_dir.path().to_owned();

    println!("Creating {N_FILES} files for the test site...");

    // Step 1: Create initial HTML files
    fs::create_dir_all(&test_site_dir)?;
    for i in 0..N_FILES {
        let file_path = test_site_dir.join(format!("file_{i}.html"));
        let mut file = File::create(file_path)?;
        writeln!(file, "<html><body><h1>File {i}</h1></body></html>")?;
    }

    println!("Publishing initial site with {N_FILES} files...");

    // Step 2: Publish the initial site
    let publish_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Publish {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(test_site_dir.clone())
                .with_epoch_count_or_max(EpochCountOrMax::Epochs(1.try_into()?))
                .build()?,
            site_name: Some("Test Update With Rename".to_string()),
        })
        .with_gas_budget(10_000_000_000)
        .build()?;

    site_builder::run(publish_args).await?;

    // Get the site object ID from the published site
    let site = cluster.last_site_created().await?;
    let site_object_id = *site.id.object_id();

    println!("Published site with object ID: {site_object_id}");

    // Verify initial publish worked correctly
    let initial_resources = cluster.site_resources(site_object_id).await?;
    assert_eq!(initial_resources.len(), N_FILES);
    println!(
        "Verified {} resources in initial site",
        initial_resources.len()
    );

    // Step 3: Store the original content of file_0.html before renaming
    let old_path = test_site_dir.join("file_0.html");
    let original_file_0_content = fs::read_to_string(&old_path)?;

    // Rename one file (file_0.html -> renamed_file.html)
    println!("Renaming file_0.html to renamed_file.html...");
    let new_path = test_site_dir.join("renamed_file.html");
    fs::rename(&old_path, &new_path)?;

    // Step 4: Update the site with the renamed file
    println!("Updating site with renamed file...");
    let update_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Update {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(test_site_dir)
                .with_epoch_count_or_max(EpochCountOrMax::Epochs(20.try_into()?))
                .build()?,
            object_id: site_object_id,
        })
        .with_gas_budget(10_000_000_000)
        .build()?;

    site_builder::run(update_args).await?;

    println!("Successfully updated site with renamed file");

    // Step 5: Verify the resources through site_resources method
    let updated_resources = cluster.site_resources(site_object_id).await?;
    assert_eq!(updated_resources.len(), N_FILES);

    // Verify that all resources can be read from Walrus network
    println!(
        "Verifying {} resources from Walrus network...",
        updated_resources.len()
    );
    for resource in &updated_resources {
        let content = verify_resource_and_get_content(&cluster, resource).await?;

        // For the renamed file, verify it has the same content as the original file_0.html
        if resource.path == "/renamed_file.html" {
            let content_str = String::from_utf8_lossy(&content);
            assert_eq!(
                content_str, original_file_0_content,
                "renamed_file.html should have the same content as the original file_0.html"
            );
            println!("✓ Verified renamed_file.html has the same content as original file_0.html");
        }
    }
    println!("Successfully verified all resources from Walrus network");

    // Verify that the old path is gone and the new path exists
    let paths: Vec<String> = updated_resources.iter().map(|r| r.path.clone()).collect();
    assert!(
        !paths.contains(&"/file_0.html".to_string()),
        "Old path /file_0.html should not exist after rename"
    );
    assert!(
        paths.contains(&"/renamed_file.html".to_string()),
        "New path /renamed_file.html should exist after rename"
    );

    // Verify that all other files are still present
    for i in 1..N_FILES {
        let expected_path = format!("/file_{i}.html");
        assert!(
            paths.contains(&expected_path),
            "Path {expected_path} should still exist"
        );
    }

    Ok(())
}

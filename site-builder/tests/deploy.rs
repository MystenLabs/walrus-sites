// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{
    fs::{self, File, OpenOptions},
    io::Write,
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
use helpers::{create_large_test_site, verify_resource_and_get_content};
use site_builder::MAX_IDENTIFIER_SIZE;

/// Test 1: Deploy command with automatic site detection (object_id = None)
///
/// This test verifies that the deploy command can automatically detect the site object ID
/// from the ws-resources.json file when object_id is not provided.
///
/// Steps:
/// 1. Publish a site using DeployQuilts
/// 2. Modify a file
/// 3. Deploy again using DeployQuilts with object_id: None (should read from ws-resources.json)
/// 4. Verify the site was updated correctly
#[tokio::test]
#[ignore]
async fn quilts_deploy_auto_update() -> anyhow::Result<()> {
    let cluster = TestSetup::start_local_test_cluster().await?;

    // Create test site with 1 file and reset ws-resources object_id
    let temp_dir = helpers::create_test_site(1)?;
    let directory = temp_dir.path().to_path_buf();
    let ws_resources_path = directory.join("ws-resources.json");

    // Step 1: Initial publish using DeployQuilts
    println!("Step 1: Publishing initial site with DeployQuilts...");
    let publish_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Deploy {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory.clone())
                .with_epoch_count_or_max(EpochCountOrMax::Epochs(1_u32.try_into().unwrap()))
                .build()?,
            site_name: None,
            object_id: None,
        })
        .build()?;
    site_builder::run(publish_args).await?;

    // Verify the site was created and ws-resources.json was updated
    let ws_resources_after_publish: WSResources =
        serde_json::from_reader(File::open(ws_resources_path.as_path())?)?;
    let site_id = *cluster.last_site_created().await?.id.object_id();
    assert_eq!(
        ws_resources_after_publish.object_id.unwrap(),
        site_id,
        "ws-resources.json should contain the site object ID"
    );
    println!("Published site with object ID: {site_id}");

    // Store original file_0.html content
    let original_index_content = fs::read_to_string(temp_dir.path().join("file_0.html"))?;
    let original_line_count = original_index_content.lines().count();

    // Step 2: Modify a file
    println!("Step 2: Modifying file_0.html...");
    let index_html_path = temp_dir.path().join("file_0.html");
    {
        let mut index_html = OpenOptions::new().append(true).open(index_html_path)?;
        writeln!(&mut index_html, "<!-- Updated by deploy test -->")?;
    }

    // Step 3: Deploy again with object_id: None (should auto-detect from ws-resources.json)
    println!("Step 3: Deploying with DeployQuilts (auto-detect site ID)...");
    let deploy_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Deploy {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory)
                .with_epoch_count_or_max(EpochCountOrMax::Max)
                .build()?,
            site_name: None,
            object_id: None, // Should read from ws-resources.json
        })
        .build()?;
    site_builder::run(deploy_args).await?;

    // Step 4: Verify the update worked
    println!("Step 4: Verifying the update...");
    let updated_site = cluster.last_site_created().await?;
    let updated_resources = cluster.site_resources(*updated_site.id.object_id()).await?;

    // The site should still have the same object ID
    assert_eq!(
        site_id,
        *updated_site.id.object_id(),
        "Site object ID should remain unchanged"
    );

    // Verify that all resources have valid hashes
    for resource in updated_resources {
        let data = verify_resource_and_get_content(&cluster, &resource).await?;

        // For file_0.html, verify it has exactly one more line than the original
        if resource.path == "/file_0.html" {
            let content = String::from_utf8_lossy(&data);
            let updated_line_count = content.lines().count();
            assert_eq!(
                updated_line_count,
                original_line_count + 1,
                "file_0.html should have exactly one more line after deploy. Original: {original_line_count}, Updated: {updated_line_count}",
            );
            assert!(
                content.contains("Updated by deploy test"),
                "file_0.html should contain the test marker"
            );
        }
    }

    println!("quilts_deploy_auto_update completed successfully");

    Ok(())
}

/// Test: Deploy command with 1001 temporary files
///
/// This test verifies that the deploy command can handle publishing and deploying
/// a site with 1001 files and it also deletes it.
///
/// Steps:
/// 1. Create a site with 1001 files
/// 2. Publish the site using Deploy command
/// 3. Verify all 1001 resources were published
/// 4. Delete the site, the resources and the routes using the Destroy command
/// 5. Verify that this site no longer exists..
#[tokio::test]
#[ignore]
async fn deploy_and_destroy_site_with_1001_files() -> anyhow::Result<()> {
    let mut cluster = TestSetup::start_local_test_cluster().await?;

    // Create test site with 1001 files
    let temp_dir = helpers::create_test_site(1001)?;
    let directory = temp_dir.path().to_path_buf();

    // Step 1: Initial publish with 1001 files
    println!("Step 1: Publishing initial site with 1001 files...");
    let publish_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Deploy {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory.clone())
                .with_epoch_count_or_max(EpochCountOrMax::Epochs(1_u32.try_into().unwrap()))
                .build()?,
            site_name: Some("1001 Files Test Site".to_string()),
            object_id: None,
        })
        .with_gas_budget(100_000_000_000)
        .build()?;
    site_builder::run(publish_args).await?;

    let site_id = *cluster.last_site_created().await?.id.object_id();
    println!("Published site with object ID: {site_id}");

    // Step 2: Verify all 1001 resources were published
    println!("Step 2: Verifying all 1001 resources were published...");
    let resources = cluster.site_resources(site_id).await?;
    assert_eq!(
        resources.len(),
        1001,
        "Should have exactly 1001 resources published"
    );

    // Get wallet address for blob verification
    let wallet_address = cluster.wallet_active_address()?;

    // Get initial blob count before destroy
    let blobs_before_destroy = cluster.get_owned_blobs(wallet_address).await?;
    println!(
        "Step 3: Blob count before destroy: {}",
        blobs_before_destroy.len()
    );
    assert!(
        !blobs_before_destroy.is_empty(),
        "Should have blobs before destroy"
    );

    // Step 3: Destroy the site
    println!("Step 4: Destroying the site with object ID: {site_id}...");
    let destroy_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Destroy { object: site_id })
        .with_gas_budget(100_000_000_000)
        .build()?;
    site_builder::run(destroy_args).await?;

    // Step 4: Verify the site no longer exists
    println!("Step 5: Verifying the site no longer exists...");

    // Try to get the site object - it should be deleted
    let site_result = cluster
        .client
        .read_api()
        .get_object_with_options(
            site_id,
            sui_sdk::rpc_types::SuiObjectDataOptions::new().with_content(),
        )
        .await?;

    // The object should either not exist or be marked as deleted
    match &site_result.data {
        None => {
            println!("Site object no longer exists (None) - verification passed");
        }
        Some(obj) => {
            // Check if the object is deleted
            assert!(
                obj.content.is_none(),
                "Site object should have no content after destroy"
            );
            println!("Site object is deleted - verification passed");
        }
    }

    // Verify that resources no longer exist
    let resources_after_destroy = cluster.site_resources(site_id).await?;
    assert_eq!(
        resources_after_destroy.len(),
        0,
        "Should have no resources after destroy"
    );

    // Verify that blobs were deleted
    let blobs_after_destroy = cluster.get_owned_blobs(wallet_address).await?;
    println!("Blob count after destroy: {}", blobs_after_destroy.len());
    assert!(
        blobs_after_destroy.len() < blobs_before_destroy.len(),
        "Blob count should decrease after destroy. Before: {}, After: {}",
        blobs_before_destroy.len(),
        blobs_after_destroy.len()
    );

    println!("deploy_and_destroy_site_with_1001_files completed successfully");

    Ok(())
}

/// Test 2: Deploy command with explicit object_id
///
/// This test verifies that the deploy command can update a site using an explicitly provided
/// object_id, overriding the one in ws-resources.json.
///
/// Steps:
/// 1. Publish two sites
/// 2. Deploy to the first site using DeployQuilts with explicit object_id of the first site
/// 3. Verify that the first site was updated (not the second one)
#[tokio::test]
#[ignore]
async fn quilts_deploy_with_explicit_object_id() -> anyhow::Result<()> {
    let cluster = TestSetup::start_local_test_cluster().await?;

    // Create test site for first site (1 file)
    let temp_dir_1 = helpers::create_test_site(1)?;
    let directory_1 = temp_dir_1.path().to_path_buf();

    // Create test site for second site (1 file)
    let temp_dir_2 = helpers::create_test_site(1)?;
    let directory_2 = temp_dir_2.path().to_path_buf();
    let ws_resources_path_2 = directory_2.join("ws-resources.json");

    // Modify site 2's file_0.html to make it different from site 1
    // This ensures the two sites don't share the same blobs
    let index_html_path_2_initial = temp_dir_2.path().join("file_0.html");
    {
        let mut index_html = OpenOptions::new()
            .append(true)
            .open(index_html_path_2_initial)?;
        writeln!(&mut index_html, "<!-- Initial site 2 marker -->")?;
    }

    // Step 1: Publish the first site
    println!("Step 1a: Publishing first site...");
    let publish_args_1 = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Deploy {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory_1.clone())
                .with_epoch_count_or_max(EpochCountOrMax::Epochs(1_u32.try_into().unwrap()))
                .build()?,
            site_name: Some("First Site".to_string()),
            object_id: None,
        })
        .build()?;
    site_builder::run(publish_args_1).await?;

    let site_1_id = *cluster.last_site_created().await?.id.object_id();
    println!("Published first site with object ID: {site_1_id}");

    // Get the initial resources of the first site
    let site_1_resources_initial = cluster.site_resources(site_1_id).await?;
    let site_1_initial_count = site_1_resources_initial.len();

    // Step 2: Publish the second site
    println!("Step 1b: Publishing second site...");
    let publish_args_2 = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Deploy {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory_2.clone())
                .with_epoch_count_or_max(EpochCountOrMax::Epochs(1_u32.try_into().unwrap()))
                .build()?,
            site_name: Some("Second Site".to_string()),
            object_id: None,
        })
        .build()?;
    site_builder::run(publish_args_2).await?;

    let site_2_id = *cluster.last_site_created().await?.id.object_id();
    println!("Published second site with object ID: {site_2_id}");

    // Verify we have two different sites
    assert_ne!(
        site_1_id, site_2_id,
        "The two sites should have different object IDs"
    );

    // Verify ws-resources.json for site 2 contains site_2_id
    let ws_resources_site_2: WSResources =
        serde_json::from_reader(File::open(ws_resources_path_2.as_path())?)?;
    assert_eq!(
        ws_resources_site_2.object_id.unwrap(),
        site_2_id,
        "Second site's ws-resources.json should contain site_2_id"
    );

    // Step 3: Modify site 1's files
    println!("Step 2: Modifying first site's file_0.html...");
    let index_html_path_1 = temp_dir_1.path().join("file_0.html");
    {
        let mut index_html = OpenOptions::new().append(true).open(index_html_path_1)?;
        writeln!(&mut index_html, "<!-- Updated first site -->")?;
    }

    // Step 4: Deploy to site 1 using directory_2 but with explicit object_id of site 1
    // This simulates updating site 1 from site 2's working directory
    println!(
        "Step 3: Deploying to first site using second site's directory but explicit object_id..."
    );

    // Modify site 2's file_0.html to make it different
    let index_html_path_2 = temp_dir_2.path().join("file_0.html");
    {
        let mut index_html = OpenOptions::new().append(true).open(index_html_path_2)?;
        writeln!(&mut index_html, "<!-- Updated second site content -->")?;
    }

    let deploy_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Deploy {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory_2.clone())
                .with_epoch_count_or_max(EpochCountOrMax::Max)
                .build()?,
            site_name: None,
            object_id: Some(site_1_id), // Explicitly target site 1
        })
        .build()?;
    site_builder::run(deploy_args).await?;

    // Step 5: Verify that site 1 was updated (not site 2)
    println!("Step 4: Verifying that the first site was updated...");

    let site_1_resources_updated = cluster.site_resources(site_1_id).await?;

    // The first site should still have the same resource count
    assert_eq!(
        site_1_resources_updated.len(),
        site_1_initial_count,
        "First site should have the same number of resources"
    );

    // Verify that site 1's file_0.html now contains the content from directory_2
    for resource in site_1_resources_updated {
        let data = verify_resource_and_get_content(&cluster, &resource).await?;

        if resource.path == "/file_0.html" {
            let content = String::from_utf8_lossy(&data);
            assert!(
                content.contains("Updated second site content"),
                "First site's file_0.html should now contain content from second directory"
            );
            assert!(
                !content.contains("Updated first site"),
                "First site's file_0.html should not contain the old modification"
            );
        }
    }

    // Verify that site 2 was NOT updated
    println!("Step 5: Verifying that the second site was NOT updated...");
    let site_2_resources = cluster.site_resources(site_2_id).await?;

    for resource in site_2_resources {
        let data = verify_resource_and_get_content(&cluster, &resource).await?;

        if resource.path == "/file_0.html" {
            let content = String::from_utf8_lossy(&data);
            // Site 2 should still have its original content with the initial marker
            // but NOT the "Updated second site content" marker
            assert!(
                content.contains("Initial site 2 marker"),
                "Second site should have its initial marker"
            );
            assert!(
                !content.contains("Updated second site content"),
                "Second site should not have been updated with the new content"
            );
        }
    }

    println!("quilts_deploy_with_explicit_object_id completed successfully");

    Ok(())
}

/// Test 3: Deploy command updates site name on Sui via ws-resources.json
///
/// This test verifies that the site name is updated on Sui when ws-resources.json
/// is modified to contain a different site_name.
///
/// Steps:
/// 1. Publish a site with an initial name using DeployQuilts
/// 2. Modify ws-resources.json to have a different site_name
/// 3. Deploy again and verify the site name on Sui was updated
#[tokio::test]
#[ignore]
async fn quilts_deploy_updates_site_name() -> anyhow::Result<()> {
    let cluster = TestSetup::start_local_test_cluster().await?;

    // Create test site with 1 file
    let temp_dir = helpers::create_test_site(1)?;
    let directory = temp_dir.path().to_path_buf();

    // Step 1: Initial publish with first name
    println!("Step 1: Publishing initial site with name 'Initial Site Name'...");
    let publish_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Deploy {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory.clone())
                .with_epoch_count_or_max(EpochCountOrMax::Epochs(1_u32.try_into().unwrap()))
                .build()?,
            site_name: Some("Initial Site Name".to_string()),
            object_id: None,
        })
        .build()?;
    site_builder::run(publish_args).await?;

    let site_id = *cluster.last_site_created().await?.id.object_id();
    println!("Published site with object ID: {site_id}");

    // Verify initial name
    let initial_site = cluster.last_site_created().await?;
    assert_eq!(
        initial_site.name, "Initial Site Name",
        "Initial site name should be 'Initial Site Name'"
    );

    // Step 2: Modify ws-resources.json to have a different site_name, then deploy
    println!(
        "Step 2: Modifying ws-resources.json site_name to 'Updated Site Name' and deploying..."
    );

    // Modify a file to trigger update
    let index_html_path = temp_dir.path().join("file_0.html");
    {
        let mut index_html = OpenOptions::new().append(true).open(index_html_path)?;
        writeln!(&mut index_html, "<!-- Modified for name update test -->")?;
    }

    // Update the site_name in ws-resources.json
    // TODO(fix): argument should take precedence from ws-resources.json, not the other way around.
    // Relevant to #SEW-462
    let ws_resources_path = directory.join("ws-resources.json");
    let mut ws_resources: WSResources = serde_json::from_reader(File::open(&ws_resources_path)?)?;
    ws_resources.site_name = Some("Updated Site Name".to_string());
    serde_json::to_writer_pretty(File::create(&ws_resources_path)?, &ws_resources)?;

    let deploy_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Deploy {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory)
                .with_epoch_count_or_max(EpochCountOrMax::Max)
                .build()?,
            site_name: None, // Let it read from ws-resources.json. TODO(fix): argument should take
            // precedence from ws-resources.json, not the other way around.
            object_id: None,
        })
        .build()?;
    site_builder::run(deploy_args).await?;

    // Step 3: Verify the name was updated on Sui
    println!("Step 3: Verifying site name was updated on Sui...");
    let updated_site = cluster.last_site_created().await?;

    // Verify it's the same site (same object ID)
    assert_eq!(
        site_id,
        *updated_site.id.object_id(),
        "Site object ID should remain unchanged"
    );

    // Verify the name was updated
    assert_eq!(
        updated_site.name, "Updated Site Name",
        "Site name should be updated to 'Updated Site Name'"
    );

    println!("quilts_deploy_updates_site_name completed successfully");

    Ok(())
}

#[tokio::test]
#[ignore]
async fn deploy_quilts_with_slot_sized_files() -> anyhow::Result<()> {
    let mut cluster = TestSetup::start_local_test_cluster().await?;
    let n_shards = cluster.cluster_state.walrus_cluster.n_shards;

    // Calculate capacity per column (slot) in bytes
    let (n_rows, n_cols) = walrus_core::encoding::source_symbols_for_n_shards(n_shards);
    let max_symbol_size = walrus_core::DEFAULT_ENCODING.max_symbol_size() as usize;
    let column_capacity = max_symbol_size * n_rows.get() as usize;

    // Available columns: n_cols - 1 (reserve 1 for quilt index)
    let available_columns = n_cols.get() as usize - 1;
    let n_files = available_columns;

    // Calculate the size for each file to exactly fill one slot
    // Each file needs (MAX_IDENTIFIER_SIZE + 8) bytes of overhead for the quilt patch header
    let file_size = column_capacity - (MAX_IDENTIFIER_SIZE + 8);

    println!("Testing full quilt with {n_files} files, each filling one slot");
    println!("File size: {file_size} bytes, Column capacity: {column_capacity} bytes");

    let temp_dir = tempfile::tempdir()?;
    create_large_test_site(temp_dir.path(), n_files, file_size)?;

    let args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Publish {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(temp_dir.path().to_owned())
                .with_epoch_count_or_max(EpochCountOrMax::Max)
                .build()?,
            site_name: None,
        })
        .with_gas_budget(100_000_000_000)
        .build()?;

    site_builder::run(args).await?;

    let site = cluster.last_site_created().await?;
    let resources = cluster.site_resources(*site.id.object_id()).await?;

    println!(
        "Successfully published site with object ID: {}",
        site.id.object_id()
    );
    assert_eq!(resources.len(), n_files);

    let wallet_address = cluster.wallet_active_address()?;
    let blobs = cluster.get_owned_blobs(wallet_address).await?;
    assert_eq!(
        blobs.len(),
        1,
        "Should have exactly 1 quilt containing all {n_files} files"
    );

    // Verify all resources are in the same quilt (same blob_id)
    let first_blob_id = resources[0].blob_id;
    for resource in &resources {
        assert_eq!(
            resource.blob_id, first_blob_id,
            "All resources should be in the same quilt"
        );
    }

    // Verify each resource can be read back
    for resource in &resources {
        verify_resource_and_get_content(&cluster, resource).await?;
    }

    println!("Successfully verified {n_files} slot-sized files in a single full quilt");

    // Step 2: Deploy new content with larger files (column_capacity - MAX_IDENTIFIER_SIZE)
    // This will cause files to span multiple slots, creating 2 quilts instead of 1
    let site_id = *site.id.object_id();
    let initial_blob_id = first_blob_id;

    let new_file_size = column_capacity - MAX_IDENTIFIER_SIZE;
    println!("\nStep 2: Deploying new site with {n_files} files of size {new_file_size} bytes...");
    println!("This should create 2 quilts instead of 1");

    let temp_dir_new = tempfile::tempdir()?;
    create_large_test_site(temp_dir_new.path(), n_files, new_file_size)?;

    let deploy_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Deploy {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(temp_dir_new.path().to_owned())
                .with_epoch_count_or_max(EpochCountOrMax::Max)
                .build()?,
            site_name: None,
            object_id: Some(site_id),
        })
        .with_gas_budget(100_000_000_000)
        .build()?;

    site_builder::run(deploy_args).await?;

    // Verify only 2 blob objects remain
    let final_blobs = cluster.get_owned_blobs(wallet_address).await?;
    println!("Final blob count: {}", final_blobs.len());
    assert_eq!(
        final_blobs.len(),
        2,
        "Should have exactly 2 blobs after deploy (old quilt deleted, 2 new quilts created)"
    );

    // Verify resources and their blob_ids
    let updated_resources = cluster.site_resources(site_id).await?;
    assert_eq!(updated_resources.len(), n_files);

    // Collect unique blob_ids from resources
    let resource_blob_ids: std::collections::HashSet<_> =
        updated_resources.iter().map(|r| r.blob_id).collect();

    println!("Resource blob_ids: {resource_blob_ids:?}");
    assert_eq!(
        resource_blob_ids.len(),
        2,
        "Resources should reference exactly 2 different blob_ids"
    );

    // Verify none of the resources use the old blob_id
    for resource in &updated_resources {
        assert_ne!(
            resource.blob_id, initial_blob_id,
            "No resource should reference the old blob_id after deploy"
        );
    }

    // Verify that resource blob_ids match owned blobs
    let owned_blob_ids: std::collections::HashSet<_> =
        final_blobs.iter().map(|b| b.blob_id.0).collect();
    for blob_id in &resource_blob_ids {
        assert!(
            owned_blob_ids.contains(&blob_id.0),
            "Resource blob_id {blob_id:?} should be in owned blobs"
        );
    }

    // Verify each resource can be read back
    println!("Verifying all resources can be read back after deploy...");
    for resource in &updated_resources {
        verify_resource_and_get_content(&cluster, resource).await?;
    }

    println!("Successfully verified deployment replaced 1 quilt with 2 quilts");

    Ok(())
}

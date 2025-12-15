// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for site redeployment with quilt management.
//!
//! These tests verify that quilts are correctly created, updated, and deleted
//! during site redeployment operations.

use std::{
    collections::HashSet,
    fs::{self, File},
    io::Write,
};

use site_builder::args::{Commands, EpochCountOrMax};
use walrus_sdk::core::BlobId;

#[allow(dead_code)]
mod localnode;
use localnode::{
    args_builder::{ArgsBuilder, PublishOptionsBuilder},
    TestSetup,
};

#[allow(dead_code)]
mod helpers;
use helpers::{create_test_site, get_quilt_identifiers, verify_resource_and_get_content};

/// Test 1: Deploy 1 file, then update to add a 2nd file and delete the 1st file.
///
/// Verifies:
/// - After the update/redeploy, only 1 quilt should exist (owned by wallet)
/// - The quilt should contain only the 2nd file
/// - The site should have a single resource pointing to the 2nd file
/// - The resource content and patch ID are correct
#[tokio::test]
#[ignore]
async fn redeploy_replace_single_file() -> anyhow::Result<()> {
    let mut cluster = TestSetup::start_local_test_cluster().await?;
    let wallet_address = cluster.wallet_active_address()?;

    // Create initial site with 1 file (file_0.html)
    let temp_dir = create_test_site(1)?;
    let directory = temp_dir.path().to_path_buf();

    // Step 1: Deploy initial site with file_0
    println!("Step 1: Deploying initial site with file_0.html...");
    let deploy_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Deploy {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory.clone())
                .with_epoch_count_or_max(EpochCountOrMax::Epochs(10_u32.try_into()?))
                .build()?,
            site_name: Some("Test Replace Single File".to_string()),
            object_id: None,
        })
        .build()?;
    site_builder::run(deploy_args).await?;

    let site_id = *cluster.last_site_created().await?.id.object_id();
    println!("Deployed site with object ID: {site_id}");

    // Verify initial state
    let initial_resources = cluster.site_resources(site_id).await?;
    assert_eq!(
        initial_resources.len(),
        1,
        "Initial site should have 1 resource"
    );
    assert_eq!(initial_resources[0].path, "/file_0.html");

    let initial_blobs = cluster.get_owned_blobs(wallet_address).await?;
    assert_eq!(
        initial_blobs.len(),
        1,
        "Initial deploy should create 1 quilt"
    );

    // Step 2: Replace file_0 with file_1
    println!("Step 2: Replacing file_0.html with file_1.html...");
    fs::remove_file(directory.join("file_0.html"))?;
    {
        let mut file = File::create(directory.join("file_1.html"))?;
        writeln!(
            file,
            "<html><body><h1>File 1</h1><p>This is file 1 content</p></body></html>"
        )?;
    }

    // Step 3: Redeploy
    println!("Step 3: Redeploying site...");
    let redeploy_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Deploy {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory)
                .with_epoch_count_or_max(EpochCountOrMax::Epochs(10_u32.try_into()?))
                .build()?,
            site_name: None,
            object_id: Some(site_id),
        })
        .build()?;
    site_builder::run(redeploy_args).await?;

    // Step 4: Verify results
    println!("Step 4: Verifying results...");

    // Check owned blobs: should be exactly 1
    let final_blobs = cluster.get_owned_blobs(wallet_address).await?;
    assert_eq!(
        final_blobs.len(),
        1,
        "After redeploy, should have exactly 1 quilt owned by wallet"
    );

    // Check site resources: should be exactly 1, pointing to file_1
    let final_resources = cluster.site_resources(site_id).await?;
    assert_eq!(
        final_resources.len(),
        1,
        "After redeploy, site should have exactly 1 resource"
    );
    assert_eq!(
        final_resources[0].path, "/file_1.html",
        "Resource should be file_1.html"
    );

    // Verify resource references the owned quilt
    let resource_blob_id = final_resources[0].blob_id;
    assert_eq!(
        resource_blob_id.0, final_blobs[0].blob_id.0,
        "Resource should reference the owned quilt"
    );

    // Verify quilt metadata contains only file_1
    let quilt_blob_id = BlobId(resource_blob_id.0);
    let metadata = cluster.read_quilt_metadata(&quilt_blob_id).await?;
    let identifiers = get_quilt_identifiers(&metadata);
    assert_eq!(identifiers.len(), 1, "Quilt should contain exactly 1 file");
    assert!(
        identifiers.iter().any(|id| id.contains("file_1.html")),
        "Quilt should contain file_1.html, found: {:?}",
        identifiers
    );
    assert!(
        !identifiers.iter().any(|id| id.contains("file_0.html")),
        "Quilt should NOT contain file_0.html"
    );

    // Verify resource content and patch ID
    let resource = &final_resources[0];
    assert!(
        resource
            .headers
            .0
            .contains_key("x-wal-quilt-patch-internal-id"),
        "Resource should have quilt patch ID header"
    );
    let content = verify_resource_and_get_content(&cluster, resource).await?;
    let content_str = String::from_utf8_lossy(&content);
    assert!(
        content_str.contains("file 1 content"),
        "Resource content should be file 1's content"
    );

    println!("redeploy_replace_single_file completed successfully");
    Ok(())
}

/// Test 2: Deploy 1 file, then update the file contents.
///
/// Verifies:
/// - After the update/redeploy, only 1 quilt should exist (owned by wallet)
/// - The quilt should contain the updated file
/// - The site should have a single resource pointing to the updated file
/// - The resource content reflects the update
#[tokio::test]
#[ignore]
async fn redeploy_update_file_contents() -> anyhow::Result<()> {
    let mut cluster = TestSetup::start_local_test_cluster().await?;
    let wallet_address = cluster.wallet_active_address()?;

    // Create initial site with 1 file (file_0.html)
    let temp_dir = create_test_site(1)?;
    let directory = temp_dir.path().to_path_buf();

    // Step 1: Deploy initial site
    println!("Step 1: Deploying initial site with file_0.html...");
    let deploy_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Deploy {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory.clone())
                .with_epoch_count_or_max(EpochCountOrMax::Epochs(10_u32.try_into()?))
                .build()?,
            site_name: Some("Test Update File Contents".to_string()),
            object_id: None,
        })
        .build()?;
    site_builder::run(deploy_args).await?;

    let site_id = *cluster.last_site_created().await?.id.object_id();
    println!("Deployed site with object ID: {site_id}");

    // Verify initial state
    let initial_resources = cluster.site_resources(site_id).await?;
    assert_eq!(initial_resources.len(), 1);
    let initial_content = verify_resource_and_get_content(&cluster, &initial_resources[0]).await?;
    assert!(String::from_utf8_lossy(&initial_content).contains("Test File 0"));

    let initial_blobs = cluster.get_owned_blobs(wallet_address).await?;
    assert_eq!(
        initial_blobs.len(),
        1,
        "Initial deploy should create 1 quilt"
    );
    let initial_blob_id = initial_blobs[0].blob_id;

    // Step 2: Update file_0 content
    println!("Step 2: Updating file_0.html content...");
    fs::write(
        directory.join("file_0.html"),
        "<html><body><h1>Updated</h1><p>Updated content - modified</p></body></html>",
    )?;

    // Step 3: Redeploy
    println!("Step 3: Redeploying site...");
    let redeploy_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Deploy {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory)
                .with_epoch_count_or_max(EpochCountOrMax::Epochs(10_u32.try_into()?))
                .build()?,
            site_name: None,
            object_id: Some(site_id),
        })
        .build()?;
    site_builder::run(redeploy_args).await?;

    // Step 4: Verify results
    println!("Step 4: Verifying results...");

    // Check owned blobs: should be exactly 1
    let final_blobs = cluster.get_owned_blobs(wallet_address).await?;
    assert_eq!(
        final_blobs.len(),
        1,
        "After redeploy, should have exactly 1 quilt owned by wallet"
    );

    // Verify it's a NEW blob (different from initial)
    assert_ne!(
        final_blobs[0].blob_id, initial_blob_id,
        "After update, should have a new blob (old one deleted)"
    );

    // Check site resources: should be exactly 1
    let final_resources = cluster.site_resources(site_id).await?;
    assert_eq!(
        final_resources.len(),
        1,
        "After redeploy, site should have exactly 1 resource"
    );
    assert_eq!(final_resources[0].path, "/file_0.html");

    // Verify resource references the owned quilt
    let resource_blob_id = final_resources[0].blob_id;
    assert_eq!(
        resource_blob_id.0, final_blobs[0].blob_id.0,
        "Resource should reference the owned quilt"
    );

    // Verify quilt metadata contains file_0
    let quilt_blob_id = BlobId(resource_blob_id.0);
    let metadata = cluster.read_quilt_metadata(&quilt_blob_id).await?;
    let identifiers = get_quilt_identifiers(&metadata);
    assert_eq!(identifiers.len(), 1, "Quilt should contain exactly 1 file");
    assert!(
        identifiers.iter().any(|id| id.contains("file_0.html")),
        "Quilt should contain file_0.html"
    );

    // Verify resource content is updated and patch ID exists
    let resource = &final_resources[0];
    assert!(
        resource
            .headers
            .0
            .contains_key("x-wal-quilt-patch-internal-id"),
        "Resource should have quilt patch ID header"
    );
    let content = verify_resource_and_get_content(&cluster, resource).await?;
    let content_str = String::from_utf8_lossy(&content);
    assert!(
        content_str.contains("Updated content - modified"),
        "Resource content should be updated. Got: {}",
        content_str
    );
    assert!(
        !content_str.contains("Test File 0"),
        "Resource should NOT contain original content"
    );

    println!("redeploy_update_file_contents completed successfully");
    Ok(())
}

/// Test 3: Deploy 2 files, redeploy to change 2nd file and add 3rd file,
/// then redeploy again to delete the 1st file.
///
/// Verifies after 1st redeploy:
/// - 2 quilts owned by wallet
/// - Quilt-1 contains file_0 (unchanged)
/// - Quilt-2 contains file_1-edited and file_2-new
/// - 3 site resources: file_0 → quilt-1, file_1 → quilt-2, file_2 → quilt-2
///
/// Verifies after 2nd redeploy:
/// - 1 quilt owned by wallet (quilt-1 deleted, quilt-2 remains)
/// - Quilt-2 contains file_1-edited and file_2-new
/// - 2 site resources: file_1 and file_2, both → quilt-2
#[tokio::test]
#[ignore]
async fn redeploy_multi_step_changes() -> anyhow::Result<()> {
    let mut cluster = TestSetup::start_local_test_cluster().await?;
    let wallet_address = cluster.wallet_active_address()?;

    // Create initial site with 2 files (file_0.html and file_1.html)
    let temp_dir = create_test_site(2)?;
    let directory = temp_dir.path().to_path_buf();

    // Step 1: Deploy initial site with 2 files
    println!("Step 1: Deploying initial site with file_0.html and file_1.html...");
    let deploy_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Deploy {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory.clone())
                .with_epoch_count_or_max(EpochCountOrMax::Epochs(10_u32.try_into()?))
                .build()?,
            site_name: Some("Test Multi-Step Changes".to_string()),
            object_id: None,
        })
        .build()?;
    site_builder::run(deploy_args).await?;

    let site_id = *cluster.last_site_created().await?.id.object_id();
    println!("Deployed site with object ID: {site_id}");

    // Verify initial state
    let initial_resources = cluster.site_resources(site_id).await?;
    assert_eq!(
        initial_resources.len(),
        2,
        "Initial site should have 2 resources"
    );

    let initial_blobs = cluster.get_owned_blobs(wallet_address).await?;
    assert_eq!(
        initial_blobs.len(),
        1,
        "Initial deploy should create 1 quilt (both files in same quilt)"
    );

    // Store the initial quilt blob_id
    let initial_quilt_blob_id = initial_blobs[0].blob_id;

    // Step 2: First redeploy - Edit file_1 and add file_2 (keep file_0 unchanged)
    println!("Step 2: First redeploy - editing file_1.html and adding file_2.html...");

    // Update file_1 content
    fs::write(
        directory.join("file_1.html"),
        "<html><body><h1>file_1.html</h1><p>File 1 EDITED content</p></body></html>",
    )?;

    // Add file_2
    {
        let mut file = File::create(directory.join("file_2.html"))?;
        writeln!(
            file,
            "<html><body><h1>file_2.html</h1><p>File 2 NEW content</p></body></html>"
        )?;
    }

    let redeploy_1_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Deploy {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory.clone())
                .with_epoch_count_or_max(EpochCountOrMax::Epochs(10_u32.try_into()?))
                .build()?,
            site_name: None,
            object_id: Some(site_id),
        })
        .build()?;
    site_builder::run(redeploy_1_args).await?;

    // Verify state after first redeploy
    println!("Verifying state after first redeploy...");

    let blobs_after_redeploy_1 = cluster.get_owned_blobs(wallet_address).await?;
    assert_eq!(
        blobs_after_redeploy_1.len(),
        2,
        "After first redeploy, should have 2 quilts (original + new for changed/added files)"
    );

    let resources_after_redeploy_1 = cluster.site_resources(site_id).await?;
    assert_eq!(
        resources_after_redeploy_1.len(),
        3,
        "After first redeploy, should have 3 resources"
    );

    // Find which quilt is which by looking at the resources
    let file_0_resource = resources_after_redeploy_1
        .iter()
        .find(|r| r.path == "/file_0.html")
        .expect("file_0.html resource should exist");
    let file_1_resource = resources_after_redeploy_1
        .iter()
        .find(|r| r.path == "/file_1.html")
        .expect("file_1.html resource should exist");
    let file_2_resource = resources_after_redeploy_1
        .iter()
        .find(|r| r.path == "/file_2.html")
        .expect("file_2.html resource should exist");

    let quilt_1_blob_id = file_0_resource.blob_id;
    let quilt_2_blob_id = file_1_resource.blob_id;

    // file_0 should still reference the initial quilt
    assert_eq!(
        quilt_1_blob_id.0, initial_quilt_blob_id.0,
        "file_0 should still reference the original quilt"
    );

    // file_1 and file_2 should reference the same NEW quilt
    assert_eq!(
        file_1_resource.blob_id, file_2_resource.blob_id,
        "file_1 and file_2 should be in the same quilt"
    );
    assert_ne!(
        quilt_2_blob_id, quilt_1_blob_id,
        "file_1/file_2 quilt should be different from file_0 quilt"
    );

    // Verify quilt-1 metadata (should contain file_0)
    let quilt_1_metadata = cluster
        .read_quilt_metadata(&BlobId(quilt_1_blob_id.0))
        .await?;
    let quilt_1_identifiers = get_quilt_identifiers(&quilt_1_metadata);
    println!("Quilt-1 identifiers: {:?}", quilt_1_identifiers);
    assert!(
        quilt_1_identifiers
            .iter()
            .any(|id| id.contains("file_0.html")),
        "Quilt-1 should contain file_0.html"
    );

    // Verify quilt-2 metadata (should contain file_1 and file_2)
    let quilt_2_metadata = cluster
        .read_quilt_metadata(&BlobId(quilt_2_blob_id.0))
        .await?;
    let quilt_2_identifiers = get_quilt_identifiers(&quilt_2_metadata);
    println!("Quilt-2 identifiers: {:?}", quilt_2_identifiers);
    assert_eq!(
        quilt_2_identifiers.len(),
        2,
        "Quilt-2 should contain exactly 2 files"
    );
    assert!(
        quilt_2_identifiers
            .iter()
            .any(|id| id.contains("file_1.html")),
        "Quilt-2 should contain file_1.html"
    );
    assert!(
        quilt_2_identifiers
            .iter()
            .any(|id| id.contains("file_2.html")),
        "Quilt-2 should contain file_2.html"
    );

    // Verify resource contents and patch IDs
    for resource in &resources_after_redeploy_1 {
        assert!(
            resource
                .headers
                .0
                .contains_key("x-wal-quilt-patch-internal-id"),
            "Resource {} should have quilt patch ID header",
            resource.path
        );
        verify_resource_and_get_content(&cluster, resource).await?;
    }

    // Verify file_1 has edited content
    let file_1_content = verify_resource_and_get_content(&cluster, file_1_resource).await?;
    assert!(
        String::from_utf8_lossy(&file_1_content).contains("EDITED"),
        "file_1 should have edited content"
    );

    // Verify file_2 has new content
    let file_2_content = verify_resource_and_get_content(&cluster, file_2_resource).await?;
    assert!(
        String::from_utf8_lossy(&file_2_content).contains("NEW"),
        "file_2 should have new content"
    );

    println!("First redeploy verification passed!");

    // Step 3: Second redeploy - Delete file_0
    println!("Step 3: Second redeploy - deleting file_0.html...");
    fs::remove_file(directory.join("file_0.html"))?;

    let redeploy_2_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Deploy {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory)
                .with_epoch_count_or_max(EpochCountOrMax::Epochs(10_u32.try_into()?))
                .build()?,
            site_name: None,
            object_id: Some(site_id),
        })
        .build()?;
    site_builder::run(redeploy_2_args).await?;

    // Verify final state after second redeploy
    println!("Verifying state after second redeploy...");

    let final_blobs = cluster.get_owned_blobs(wallet_address).await?;
    assert_eq!(
        final_blobs.len(),
        1,
        "After second redeploy, should have exactly 1 quilt (quilt-1 deleted since file_0 removed)"
    );

    // The remaining quilt should be quilt-2
    assert_eq!(
        final_blobs[0].blob_id.0, quilt_2_blob_id.0,
        "Remaining quilt should be quilt-2 (containing file_1 and file_2)"
    );

    let final_resources = cluster.site_resources(site_id).await?;
    assert_eq!(
        final_resources.len(),
        2,
        "After second redeploy, should have exactly 2 resources"
    );

    // Verify resource paths
    let final_paths: HashSet<_> = final_resources.iter().map(|r| r.path.as_str()).collect();
    assert!(
        final_paths.contains("/file_1.html"),
        "Should have file_1.html"
    );
    assert!(
        final_paths.contains("/file_2.html"),
        "Should have file_2.html"
    );
    assert!(
        !final_paths.contains("/file_0.html"),
        "Should NOT have file_0.html"
    );

    // Verify both resources point to quilt-2
    for resource in &final_resources {
        assert_eq!(
            resource.blob_id.0, quilt_2_blob_id.0,
            "Resource {} should reference quilt-2",
            resource.path
        );
    }

    // Verify quilt-2 metadata still contains file_1 and file_2
    let final_quilt_metadata = cluster
        .read_quilt_metadata(&BlobId(quilt_2_blob_id.0))
        .await?;
    let final_identifiers = get_quilt_identifiers(&final_quilt_metadata);
    assert_eq!(
        final_identifiers.len(),
        2,
        "Final quilt should still contain 2 files"
    );
    assert!(
        final_identifiers
            .iter()
            .any(|id| id.contains("file_1.html")),
        "Final quilt should contain file_1.html"
    );
    assert!(
        final_identifiers
            .iter()
            .any(|id| id.contains("file_2.html")),
        "Final quilt should contain file_2.html"
    );

    // Verify resource contents and patch IDs
    for resource in &final_resources {
        assert!(
            resource
                .headers
                .0
                .contains_key("x-wal-quilt-patch-internal-id"),
            "Resource {} should have quilt patch ID header",
            resource.path
        );
        verify_resource_and_get_content(&cluster, resource).await?;
    }

    println!("redeploy_multi_step_changes completed successfully");
    Ok(())
}

/// Test 4: Deploy 4 files, then redeploy where ALL original files are either
/// deleted or edited (none unchanged), plus add new files.
///
/// Initial state: 4 files (file_0, file_1, file_2, file_3) in 1 quilt
/// Redeploy with:
/// - Delete file_0 and file_1
/// - Edit file_2 and file_3 (no unchanged files!)
/// - Add file_4 and file_5
///
/// Verifies after redeploy:
/// - 1 quilt: original is deleted (no unchanged files), new quilt created
/// - 4 site resources: file_2, file_3, file_4, file_5
/// - All resources reference the new quilt
/// - Correct content for all resources
#[tokio::test]
#[ignore]
async fn redeploy_delete_some_edit_rest_add_new() -> anyhow::Result<()> {
    let mut cluster = TestSetup::start_local_test_cluster().await?;
    let wallet_address = cluster.wallet_active_address()?;

    // Create initial site with 4 files
    let temp_dir = create_test_site(4)?;
    let directory = temp_dir.path().to_path_buf();

    // Step 1: Deploy initial site with 4 files
    println!("Step 1: Deploying initial site with 4 files (file_0 to file_3)...");
    let deploy_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Deploy {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory.clone())
                .with_epoch_count_or_max(EpochCountOrMax::Epochs(10_u32.try_into()?))
                .build()?,
            site_name: Some("Test Delete Some Edit Rest".to_string()),
            object_id: None,
        })
        .build()?;
    site_builder::run(deploy_args).await?;

    let site_id = *cluster.last_site_created().await?.id.object_id();
    println!("Deployed site with object ID: {site_id}");

    // Verify initial state
    let initial_resources = cluster.site_resources(site_id).await?;
    assert_eq!(
        initial_resources.len(),
        4,
        "Initial site should have 4 resources"
    );

    let initial_blobs = cluster.get_owned_blobs(wallet_address).await?;
    assert_eq!(
        initial_blobs.len(),
        1,
        "Initial deploy should create 1 quilt"
    );
    let original_quilt_blob_id = initial_blobs[0].blob_id;

    // Step 2: Redeploy - delete some, edit the rest, add new
    println!("Step 2: Redeploying with mixed operations...");
    println!("  - Deleting file_0.html and file_1.html");
    println!("  - Editing file_2.html and file_3.html (no unchanged files!)");
    println!("  - Adding file_4.html and file_5.html");

    // Delete file_0 and file_1
    fs::remove_file(directory.join("file_0.html"))?;
    fs::remove_file(directory.join("file_1.html"))?;

    // Edit file_2 AND file_3 (no unchanged files!)
    fs::write(
        directory.join("file_2.html"),
        "<html><body><h1>file_2.html</h1><p>File 2 EDITED content</p></body></html>",
    )?;
    fs::write(
        directory.join("file_3.html"),
        "<html><body><h1>file_3.html</h1><p>File 3 EDITED content</p></body></html>",
    )?;

    // Add file_4 and file_5
    fs::write(
        directory.join("file_4.html"),
        "<html><body><h1>file_4.html</h1><p>File 4 NEW content</p></body></html>",
    )?;
    fs::write(
        directory.join("file_5.html"),
        "<html><body><h1>file_5.html</h1><p>File 5 NEW content</p></body></html>",
    )?;

    let redeploy_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Deploy {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory)
                .with_epoch_count_or_max(EpochCountOrMax::Epochs(10_u32.try_into()?))
                .build()?,
            site_name: None,
            object_id: Some(site_id),
        })
        .build()?;
    site_builder::run(redeploy_args).await?;

    // Step 3: Verify results
    println!("Step 3: Verifying results...");

    // Check owned blobs: should be 1 (original deleted since no unchanged files, new one created)
    let final_blobs = cluster.get_owned_blobs(wallet_address).await?;
    assert_eq!(
        final_blobs.len(),
        1,
        "After redeploy, should have 1 quilt (original deleted, new one created)"
    );

    // Verify it's a NEW quilt (not the original)
    assert_ne!(
        final_blobs[0].blob_id.0, original_quilt_blob_id.0,
        "Should have a new quilt, not the original (original should be deleted)"
    );
    let new_quilt_blob_id = final_blobs[0].blob_id;

    // Check site resources: should be 4 (file_2, file_3, file_4, file_5)
    let final_resources = cluster.site_resources(site_id).await?;
    assert_eq!(
        final_resources.len(),
        4,
        "After redeploy, should have exactly 4 resources"
    );

    // Verify resource paths
    let final_paths: HashSet<_> = final_resources.iter().map(|r| r.path.as_str()).collect();
    assert!(
        !final_paths.contains("/file_0.html"),
        "Should NOT have file_0.html (deleted)"
    );
    assert!(
        !final_paths.contains("/file_1.html"),
        "Should NOT have file_1.html (deleted)"
    );
    assert!(
        final_paths.contains("/file_2.html"),
        "Should have file_2.html (edited)"
    );
    assert!(
        final_paths.contains("/file_3.html"),
        "Should have file_3.html (edited)"
    );
    assert!(
        final_paths.contains("/file_4.html"),
        "Should have file_4.html (new)"
    );
    assert!(
        final_paths.contains("/file_5.html"),
        "Should have file_5.html (new)"
    );

    // All resources should reference the same new quilt
    for resource in &final_resources {
        assert_eq!(
            resource.blob_id.0, new_quilt_blob_id.0,
            "Resource {} should reference the new quilt",
            resource.path
        );
    }

    // Verify the new quilt metadata contains all 4 files
    let new_quilt_metadata = cluster
        .read_quilt_metadata(&BlobId(new_quilt_blob_id.0))
        .await?;
    let new_quilt_identifiers = get_quilt_identifiers(&new_quilt_metadata);
    println!("New quilt identifiers: {:?}", new_quilt_identifiers);
    assert_eq!(
        new_quilt_identifiers.len(),
        4,
        "New quilt should contain exactly 4 files"
    );
    for filename in ["file_2.html", "file_3.html", "file_4.html", "file_5.html"] {
        assert!(
            new_quilt_identifiers.iter().any(|id| id.contains(filename)),
            "New quilt should contain {}",
            filename
        );
    }

    // Verify resource contents
    let file_2_resource = final_resources
        .iter()
        .find(|r| r.path == "/file_2.html")
        .unwrap();
    let file_3_resource = final_resources
        .iter()
        .find(|r| r.path == "/file_3.html")
        .unwrap();
    let file_4_resource = final_resources
        .iter()
        .find(|r| r.path == "/file_4.html")
        .unwrap();
    let file_5_resource = final_resources
        .iter()
        .find(|r| r.path == "/file_5.html")
        .unwrap();

    // Edited files should have EDITED content
    let file_2_content = verify_resource_and_get_content(&cluster, file_2_resource).await?;
    assert!(
        String::from_utf8_lossy(&file_2_content).contains("EDITED"),
        "file_2 should have edited content"
    );

    let file_3_content = verify_resource_and_get_content(&cluster, file_3_resource).await?;
    assert!(
        String::from_utf8_lossy(&file_3_content).contains("EDITED"),
        "file_3 should have edited content"
    );

    // New files should have NEW content
    let file_4_content = verify_resource_and_get_content(&cluster, file_4_resource).await?;
    assert!(
        String::from_utf8_lossy(&file_4_content).contains("NEW"),
        "file_4 should have new content"
    );

    let file_5_content = verify_resource_and_get_content(&cluster, file_5_resource).await?;
    assert!(
        String::from_utf8_lossy(&file_5_content).contains("NEW"),
        "file_5 should have new content"
    );

    // Verify all resources have patch IDs
    for resource in &final_resources {
        assert!(
            resource
                .headers
                .0
                .contains_key("x-wal-quilt-patch-internal-id"),
            "Resource {} should have quilt patch ID header",
            resource.path
        );
    }

    println!("redeploy_delete_some_edit_rest_add_new completed successfully");
    Ok(())
}

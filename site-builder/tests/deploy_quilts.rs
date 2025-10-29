// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

#![cfg(feature = "quilts-experimental")]

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
use helpers::verify_resource_and_get_content;

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
        .with_command(Commands::DeployQuilts {
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
        .with_command(Commands::DeployQuilts {
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
        .with_command(Commands::DeployQuilts {
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
        .with_command(Commands::DeployQuilts {
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
        .with_command(Commands::DeployQuilts {
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
        .with_command(Commands::DeployQuilts {
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
    let index_html_path = temp_dir.path().join("0.html");
    {
        let mut index_html = OpenOptions::new().append(true).open(index_html_path)?;
        writeln!(&mut index_html, "<!-- Modified for name update test -->")?;
    }

    // Update the site_name in ws-resources.json
    // TODO(fix): argument should take precedence from ws-resources.json, not the other way around.
    let ws_resources_path = directory.join("ws-resources.json");
    let mut ws_resources: WSResources = serde_json::from_reader(File::open(&ws_resources_path)?)?;
    ws_resources.site_name = Some("Updated Site Name".to_string());
    serde_json::to_writer_pretty(File::create(&ws_resources_path)?, &ws_resources)?;

    let deploy_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::DeployQuilts {
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

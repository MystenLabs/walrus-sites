// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{fs::File, io::Write};

use site_builder::args::{Commands, EpochCountOrMax, ResourcePaths};

#[allow(dead_code)]
mod localnode;
use localnode::{
    args_builder::{ArgsBuilder, PublishOptionsBuilder, WalrusStoreOptionsBuilder},
    TestSetup,
};

#[allow(dead_code)]
mod helpers;
use helpers::{create_test_site, verify_resource_and_get_content};

#[tokio::test]
#[ignore]
async fn test_update_resources_add_files() -> anyhow::Result<()> {
    let cluster = TestSetup::start_local_test_cluster(None).await?;

    // 1. Generate a dummy site with initial files
    let temp_dir = create_test_site(3)?;
    let directory = temp_dir.path().to_path_buf();

    // Publish the initial site
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

    let site_id = *cluster.last_site_created().await?.id.object_id();

    // 2. Create two new files to add via update-resources
    let new_file1_path = directory.join("new_file1.html");
    let mut new_file1 = File::create(&new_file1_path)?;
    writeln!(new_file1, "<html><body><h1>New File 1</h1></body></html>")?;
    drop(new_file1);

    let new_file2_path = directory.join("new_file2.html");
    let mut new_file2 = File::create(&new_file2_path)?;
    writeln!(new_file2, "<html><body><h1>New File 2</h1></body></html>")?;
    drop(new_file2);

    // 3. Call update-resources to add the two new files
    let update_resources_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::UpdateResources {
            resources: vec![
                ResourcePaths {
                    file_path: new_file1_path.clone(),
                    url_path: "/new_file1.html".to_string(),
                },
                ResourcePaths {
                    file_path: new_file2_path.clone(),
                    url_path: "/new_file2.html".to_string(),
                },
            ],
            site_object: site_id,
            common: WalrusStoreOptionsBuilder::default()
                .with_epoch_count_or_max(EpochCountOrMax::Epochs(1.try_into()?))
                .build()?,
        })
        .build()?;
    site_builder::run(update_resources_args).await?;

    // 4. Verify that all files including the added ones are present and valid
    let updated_resources = cluster.site_resources(site_id).await?;

    // Should have 5 files now (3 original + 2 new)
    assert_eq!(
        updated_resources.len(),
        5,
        "Expected 5 resources after adding two"
    );

    // Verify that all resources have valid hashes
    for resource in &updated_resources {
        let _data = verify_resource_and_get_content(&cluster, resource).await?;
    }

    // Verify the new files are present
    let new_resource = updated_resources
        .iter()
        .find(|r| r.path == "/new_file1.html")
        .expect("New resource should be present");

    let new_content = verify_resource_and_get_content(&cluster, new_resource).await?;
    let new_content_str = String::from_utf8(new_content)?;
    assert!(
        new_content_str.contains("New File 1"),
        "New file content should match"
    );
    let new_resource = updated_resources
        .iter()
        .find(|r| r.path == "/new_file2.html")
        .expect("New resource should be present");

    let new_content = verify_resource_and_get_content(&cluster, new_resource).await?;
    let new_content_str = String::from_utf8(new_content)?;
    assert!(
        new_content_str.contains("New File 2"),
        "New file content should match"
    );

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_update_resources_add_single_file() -> anyhow::Result<()> {
    let cluster = TestSetup::start_local_test_cluster(None).await?;

    // 1. Generate a dummy site with initial files
    let temp_dir = create_test_site(3)?;
    let directory = temp_dir.path().to_path_buf();

    // Publish the initial site
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

    let site_id = *cluster.last_site_created().await?.id.object_id();

    // 2. Create a single new file to add
    let new_file_path = directory.join("new_file.html");
    let mut new_file = File::create(&new_file_path)?;
    writeln!(
        new_file,
        "<html><body><h1>New Single File</h1></body></html>"
    )?;
    drop(new_file);

    // 3. Call update-resources to add the single new file
    let update_resources_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::UpdateResources {
            resources: vec![ResourcePaths {
                file_path: new_file_path.clone(),
                url_path: "/new_file.html".to_string(),
            }],
            site_object: site_id,
            common: WalrusStoreOptionsBuilder::default()
                .with_epoch_count_or_max(EpochCountOrMax::Epochs(1.try_into()?))
                .build()?,
        })
        .build()?;
    site_builder::run(update_resources_args).await?;

    // 4. Verify that the site now has 4 files (3 original + 1 new)
    let updated_resources = cluster.site_resources(site_id).await?;

    assert_eq!(
        updated_resources.len(),
        4,
        "Expected 4 resources after adding one"
    );

    // Verify that all resources have valid hashes
    for resource in &updated_resources {
        let _data = verify_resource_and_get_content(&cluster, resource).await?;
    }

    // Verify the new file is present with correct content
    let new_resource = updated_resources
        .iter()
        .find(|r| r.path == "/new_file.html")
        .expect("New resource should be present");

    let new_content = verify_resource_and_get_content(&cluster, new_resource).await?;
    let new_content_str = String::from_utf8(new_content)?;
    assert!(
        new_content_str.contains("New Single File"),
        "New file content should match"
    );

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_update_resources_replace_file() -> anyhow::Result<()> {
    let cluster = TestSetup::start_local_test_cluster(None).await?;

    // 1. Generate a dummy site with initial files
    let temp_dir = create_test_site(3)?;
    let directory = temp_dir.path().to_path_buf();

    // Publish the initial site
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

    let site_id = *cluster.last_site_created().await?.id.object_id();

    // Get the original resources to verify replacement later
    let original_resources = cluster.site_resources(site_id).await?;
    assert_eq!(original_resources.len(), 3, "Expected 3 initial resources");

    // Find the original file_1.html resource
    let original_file_1 = original_resources
        .iter()
        .find(|r| r.path == "/file_1.html")
        .expect("Original file_1.html should exist");

    // 2. Create a new file with updated content
    let updated_file_path = directory.join("updated_file.html");
    writeln!(
        File::create(&updated_file_path)?,
        "<html><body><h1>Updated Content</h1></body></html>"
    )?;

    // 3. Call update-resources to replace file_1.html with the new content
    let update_resources_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::UpdateResources {
            resources: vec![ResourcePaths {
                file_path: updated_file_path.clone(),
                url_path: "/file_1.html".to_string(),
            }],
            site_object: site_id,
            common: WalrusStoreOptionsBuilder::default()
                .with_epoch_count_or_max(EpochCountOrMax::Epochs(1.try_into()?))
                .build()?,
        })
        .build()?;
    site_builder::run(update_resources_args).await?;

    // 4. Verify that we still have 3 files (resource was replaced, not added)
    let updated_resources = cluster.site_resources(site_id).await?;
    assert_eq!(
        updated_resources.len(),
        3,
        "Expected 3 resources after replacement (same count)"
    );

    // Verify that all resources have valid hashes
    for resource in &updated_resources {
        let _data = verify_resource_and_get_content(&cluster, resource).await?;
    }

    // Verify the replaced file has new content
    let replaced_resource = updated_resources
        .iter()
        .find(|r| r.path == "/file_1.html")
        .expect("Replaced resource should be present");

    let replaced_content = verify_resource_and_get_content(&cluster, replaced_resource).await?;
    let replaced_content_str = String::from_utf8(replaced_content)?;
    assert!(
        replaced_content_str.contains("Updated Content"),
        "Replaced file should have new content"
    );
    assert!(
        !replaced_content_str.contains("Test File 1"),
        "Replaced file should not have old content"
    );

    // Verify the blob ID changed (since content changed)
    assert_ne!(
        original_file_1.blob_id, replaced_resource.blob_id,
        "Blob ID should change when content is updated"
    );

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_update_resources_add_and_replace() -> anyhow::Result<()> {
    let cluster = TestSetup::start_local_test_cluster(None).await?;

    // 1. Generate a dummy site with initial files
    let temp_dir = create_test_site(3)?;
    let directory = temp_dir.path().to_path_buf();

    // Publish the initial site
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

    let site_id = *cluster.last_site_created().await?.id.object_id();

    // Get the original resources to verify replacement later
    let original_resources = cluster.site_resources(site_id).await?;
    assert_eq!(original_resources.len(), 3, "Expected 3 initial resources");

    // Find the original file_0.html resource
    let original_file_0 = original_resources
        .iter()
        .find(|r| r.path == "/file_0.html")
        .expect("Original file_0.html should exist");

    // 2. Create a new file to add
    let new_file_path = directory.join("new_file.html");
    writeln!(
        File::create(&new_file_path)?,
        "<html><body><h1>Brand New File</h1></body></html>"
    )?;

    // 3. Create a replacement file for file_0.html
    let replacement_file_path = directory.join("replacement_file.html");
    writeln!(
        File::create(&replacement_file_path)?,
        "<html><body><h1>Replaced File 0</h1></body></html>"
    )?;

    // 4. Call update-resources to both add a new file and replace an existing one
    let update_resources_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::UpdateResources {
            resources: vec![
                ResourcePaths {
                    file_path: new_file_path.clone(),
                    url_path: "/new_file.html".to_string(),
                },
                ResourcePaths {
                    file_path: replacement_file_path.clone(),
                    url_path: "/file_0.html".to_string(),
                },
            ],
            site_object: site_id,
            common: WalrusStoreOptionsBuilder::default()
                .with_epoch_count_or_max(EpochCountOrMax::Epochs(1.try_into()?))
                .build()?,
        })
        .build()?;
    site_builder::run(update_resources_args).await?;

    // 5. Verify that we now have 4 files (3 original - 1 replaced + 1 new + 1 replacement)
    let updated_resources = cluster.site_resources(site_id).await?;
    assert_eq!(
        updated_resources.len(),
        4,
        "Expected 4 resources after adding one and replacing one"
    );

    // Verify that all resources have valid hashes
    for resource in &updated_resources {
        let _data = verify_resource_and_get_content(&cluster, resource).await?;
    }

    // Verify the new file is present
    let new_resource = updated_resources
        .iter()
        .find(|r| r.path == "/new_file.html")
        .expect("New resource should be present");

    let new_content = verify_resource_and_get_content(&cluster, new_resource).await?;
    let new_content_str = String::from_utf8(new_content)?;
    assert!(
        new_content_str.contains("Brand New File"),
        "New file should have correct content"
    );

    // Verify the replaced file has new content
    let replaced_resource = updated_resources
        .iter()
        .find(|r| r.path == "/file_0.html")
        .expect("Replaced resource should be present");

    let replaced_content = verify_resource_and_get_content(&cluster, replaced_resource).await?;
    let replaced_content_str = String::from_utf8(replaced_content)?;
    assert!(
        replaced_content_str.contains("Replaced File 0"),
        "Replaced file should have new content"
    );
    assert!(
        !replaced_content_str.contains("Test File 0"),
        "Replaced file should not have old content"
    );

    // Verify the blob ID changed for the replaced file
    assert_ne!(
        original_file_0.blob_id, replaced_resource.blob_id,
        "Blob ID should change when content is updated"
    );

    // Verify the other original files are still present
    assert!(
        updated_resources.iter().any(|r| r.path == "/file_1.html"),
        "file_1.html should still be present"
    );
    assert!(
        updated_resources.iter().any(|r| r.path == "/file_2.html"),
        "file_2.html should still be present"
    );

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_update_resources_respects_ws_resources() -> anyhow::Result<()> {
    let cluster = TestSetup::start_local_test_cluster(None).await?;

    // 1. Generate a dummy site with initial files
    let temp_dir = create_test_site(3)?;
    let directory = temp_dir.path().to_path_buf();

    // 2. Create a ws-resources.json file with a custom header for *.html files
    let ws_resources_path = directory.join("ws-resources.json");
    // TODO(sew-445): ignore patterns should support .gitignore format
    let ws_resources_content = r#"{
        "headers": {
            "**/*.html": {"X-Custom-Header": "CustomValue"}
        },
        "ignore": [
            "**/private/**/*"
        ]
    }"#;
    writeln!(
        File::create(&ws_resources_path)?,
        "{}",
        ws_resources_content
    )?;

    // Publish the initial site
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

    let site_id = *cluster.last_site_created().await?.id.object_id();

    // Verify original resources have the custom header
    let original_resources = cluster.site_resources(site_id).await?;
    for resource in &original_resources {
        if resource.path.ends_with(".html") {
            assert!(
                resource.headers.0.contains_key("x-custom-header"),
                "Original HTML files should have custom header"
            );
            assert_eq!(
                resource.headers.0.get("x-custom-header").unwrap(),
                "CustomValue",
                "Original HTML files should have correct custom header value"
            );
        }
    }

    // 3. Create new files in a completely different directory
    let different_temp_dir = tempfile::tempdir()?;
    let external_file_path = different_temp_dir.path().join("external_file.html");
    writeln!(
        File::create(&external_file_path)?,
        "<html><body><h1>External File</h1></body></html>"
    )?;

    // Create a file in a private subdirectory that matches the ignore pattern
    let private_dir = different_temp_dir.path().join("private");
    std::fs::create_dir(&private_dir)?;
    let ignored_file_path = private_dir.join("secret.html");
    writeln!(
        File::create(&ignored_file_path)?,
        "<html><body><h1>Private File</h1></body></html>"
    )?;

    // 4. Call update-resources to add both files (one normal, one matching ignore pattern)
    let update_resources_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::UpdateResources {
            resources: vec![
                ResourcePaths {
                    file_path: external_file_path.clone(),
                    url_path: "/external_file.html".to_string(),
                },
                // TODO(sew-480): BUG Fix, ignore should look on the full-path, not the resource-path
                ResourcePaths {
                    file_path: ignored_file_path.clone(),
                    url_path: "/private/secret.html".to_string(),
                },
            ],
            site_object: site_id,
            common: WalrusStoreOptionsBuilder::default()
                .with_ws_resources(Some(ws_resources_path.clone()))
                .with_epoch_count_or_max(EpochCountOrMax::Epochs(1.try_into()?))
                .build()?,
        })
        .build()?;
    site_builder::run(update_resources_args).await?;

    // 5. Verify that the external file was added but the ignored file was not
    let updated_resources = cluster.site_resources(site_id).await?;

    // Should have 4 files (3 original + 1 external, ignored file should be skipped)
    assert_eq!(
        updated_resources.len(),
        4,
        "Expected 4 resources (ignored file should be skipped)"
    );

    // Verify that all resources have valid hashes
    for resource in &updated_resources {
        let _data = verify_resource_and_get_content(&cluster, resource).await?;
    }

    // Verify the external file is present
    let external_resource = updated_resources
        .iter()
        .find(|r| r.path == "/external_file.html")
        .expect("External resource should be present");

    let external_content = verify_resource_and_get_content(&cluster, external_resource).await?;
    let external_content_str = String::from_utf8(external_content)?;
    assert!(
        external_content_str.contains("External File"),
        "External file should have correct content"
    );

    // Verify that the ignored file was NOT added to the site
    let ignored_resource = updated_resources
        .iter()
        .find(|r| r.path == "/private/secret.html");
    assert!(
        ignored_resource.is_none(),
        "File matching ignore pattern should not be added to the site"
    );

    // Verify the original files are still present
    assert!(
        updated_resources.iter().any(|r| r.path == "/file_0.html"),
        "file_0.html should still be present"
    );
    assert!(
        updated_resources.iter().any(|r| r.path == "/file_1.html"),
        "file_1.html should still be present"
    );
    assert!(
        updated_resources.iter().any(|r| r.path == "/file_2.html"),
        "file_2.html should still be present"
    );

    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_update_resources_filters_ws_resources_file() -> anyhow::Result<()> {
    let cluster = TestSetup::start_local_test_cluster(None).await?;

    // 1. Generate a dummy site with initial files
    let temp_dir = create_test_site(3)?;
    let directory = temp_dir.path().to_path_buf();

    // 2. Create a ws-resources.json file
    let ws_resources_path = directory.join("ws-resources.json");
    let ws_resources_content = r#"{
        "headers": {
            "**/*.html": {"X-Custom-Header": "CustomValue"}
        }
    }"#;
    writeln!(
        File::create(&ws_resources_path)?,
        "{}",
        ws_resources_content
    )?;

    // Publish the initial site
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

    let site_id = *cluster.last_site_created().await?.id.object_id();

    // 3. Try to add the ws-resources.json file itself as a resource
    let update_resources_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::UpdateResources {
            resources: vec![ResourcePaths {
                file_path: ws_resources_path.clone(),
                url_path: "/ws-resources.json".to_string(),
            }],
            site_object: site_id,
            common: WalrusStoreOptionsBuilder::default()
                .with_ws_resources(Some(ws_resources_path.clone()))
                .with_epoch_count_or_max(EpochCountOrMax::Epochs(1.try_into()?))
                .build()?,
        })
        .build()?;

    // 4. Verify that ws-resources.json is filtered out and not added
    site_builder::run(update_resources_args).await?;

    let updated_resources = cluster.site_resources(site_id).await?;

    // Should still have 3 files (ws-resources.json should be skipped)
    assert_eq!(
        updated_resources.len(),
        3,
        "Expected 3 resources (ws-resources.json should be filtered out)"
    );

    // Verify that all resources have valid hashes
    for resource in &updated_resources {
        let _data = verify_resource_and_get_content(&cluster, resource).await?;
    }

    // Verify that ws-resources.json was NOT added to the site
    let ws_resource = updated_resources
        .iter()
        .find(|r| r.path == "/ws-resources.json");
    assert!(
        ws_resource.is_none(),
        "ws-resources.json should not be added to the site"
    );

    // Verify the original files are still present
    assert!(
        updated_resources.iter().any(|r| r.path == "/file_0.html"),
        "file_0.html should still be present"
    );
    assert!(
        updated_resources.iter().any(|r| r.path == "/file_1.html"),
        "file_1.html should still be present"
    );
    assert!(
        updated_resources.iter().any(|r| r.path == "/file_2.html"),
        "file_2.html should still be present"
    );

    Ok(())
}

/// Test that update-resources does NOT extend any existing blobs.
///
/// Unlike site-update (Update command), the update-resources command only adds/replaces
/// specific resources without extending existing blobs. This test verifies that:
/// 1. After update-resources, no extend_blob transactions are made
/// 2. Original blobs keep their original end_epochs
#[tokio::test]
#[ignore]
async fn test_update_resources_does_not_extend_blobs() -> anyhow::Result<()> {
    const PUBLISH_EPOCHS: u32 = 5;
    const UPDATE_RESOURCES_EPOCHS: u32 = 50;

    let mut cluster = TestSetup::start_local_test_cluster(None).await?;

    // Create a site with 3 files
    let temp_dir = create_test_site(3)?;
    let directory = temp_dir.path().to_path_buf();

    // Publish the initial site with short epochs
    let publish_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Publish {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory.clone())
                .with_epoch_count_or_max(EpochCountOrMax::Epochs(PUBLISH_EPOCHS.try_into()?))
                .build()?,
            site_name: None,
        })
        .build()?;
    site_builder::run(publish_args).await?;

    let site_id = *cluster.last_site_created().await?.id.object_id();
    let wallet_address = cluster.wallet_active_address();

    println!("Published site with ID: {site_id}");

    // Record initial blobs and their end_epochs
    let initial_blobs = cluster.get_owned_blobs(wallet_address).await?;
    let initial_blob_epochs: std::collections::HashMap<_, _> = initial_blobs
        .iter()
        .map(|b| (b.id, b.storage.end_epoch))
        .collect();

    println!(
        "Initial blobs: {} with end_epochs: {:?}",
        initial_blobs.len(),
        initial_blob_epochs
    );

    // Create a new file to add via update-resources
    let new_file_path = directory.join("new_file.html");
    {
        let mut new_file = File::create(&new_file_path)?;
        writeln!(new_file, "<html><body><h1>New File</h1></body></html>")?;
    }

    // Add the new file via update-resources with LONGER epochs
    println!("\n=== update-resources with {UPDATE_RESOURCES_EPOCHS} epochs ===");
    let update_resources_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::UpdateResources {
            resources: vec![ResourcePaths {
                file_path: new_file_path,
                url_path: "/new_file.html".to_string(),
            }],
            site_object: site_id,
            common: WalrusStoreOptionsBuilder::default()
                .with_epoch_count_or_max(EpochCountOrMax::Epochs(
                    UPDATE_RESOURCES_EPOCHS.try_into()?,
                ))
                .build()?,
        })
        .build()?;
    site_builder::run(update_resources_args).await?;

    println!("Successfully ran update-resources");

    // Query for extend_blob transactions - should be empty for update-resources
    let extended_blob_ids = cluster.get_extended_blob_object_ids().await?;
    assert!(
        extended_blob_ids.is_empty(),
        "update-resources should NOT extend any blobs, but extended: {:?}",
        extended_blob_ids
    );

    // Verify original blobs still have their original end_epochs
    let final_blobs = cluster.get_owned_blobs(wallet_address).await?;
    for (obj_id, original_end_epoch) in &initial_blob_epochs {
        let blob = final_blobs
            .iter()
            .find(|b| b.id == *obj_id)
            .expect("original blob should still exist");
        assert_eq!(
            blob.storage.end_epoch, *original_end_epoch,
            "Original blob {} end_epoch should not change (was {}, now {})",
            obj_id, original_end_epoch, blob.storage.end_epoch
        );
    }

    println!("\n✓ Test passed: update-resources did NOT extend any existing blobs");

    Ok(())
}

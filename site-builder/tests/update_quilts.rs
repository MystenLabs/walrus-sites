// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

#![cfg(feature = "quilts-experimental")]

use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::PathBuf,
};

use fastcrypto::hash::{HashFunction, Sha256};
use hex::FromHex;
use move_core_types::u256::U256;
use site_builder::{
    args::{Commands, EpochCountOrMax},
    site_config::WSResources,
    types::SuiResource,
};

#[allow(dead_code)]
mod localnode;
use localnode::{
    args_builder::{ArgsBuilder, PublishOptionsBuilder},
    TestSetup,
};
use walrus_sdk::core::{BlobId, QuiltPatchId};

mod helpers;

/// Verifies a resource by reading its quilt patch (if available) or blob and checking the hash.
/// Returns the content data for additional verification by the caller.
async fn verify_resource_and_get_content(
    cluster: &TestSetup,
    resource: &SuiResource,
) -> anyhow::Result<Vec<u8>> {
    let blob_id = BlobId(resource.blob_id.0);
    let patch_id = resource.headers.0.get("x-wal-quilt-patch-internal-id");

    let data = match patch_id {
        Some(patch_id_hex) => {
            // Read quilt patch
            let patch_id_bytes = Vec::from_hex(patch_id_hex.trim_start_matches("0x"))
                .expect("Invalid hex in patch ID");

            let res = cluster
                .read_quilt_patches(&[QuiltPatchId {
                    patch_id_bytes,
                    quilt_id: blob_id,
                }])
                .await?;
            assert_eq!(res.len(), 1, "Should get exactly one quilt patch");
            res[0].data().to_vec()
        }
        None => {
            // Read regular blob
            cluster.read_blob(&blob_id).await?
        }
    };

    // Verify hash
    let mut hash_function = Sha256::default();
    hash_function.update(&data);
    let resource_hash: [u8; 32] = hash_function.finalize().digest;
    assert_eq!(
        resource.blob_hash,
        U256::from_le_bytes(&resource_hash),
        "Resource hash mismatch"
    );

    Ok(data)
}

#[tokio::test]
#[ignore]
async fn quilts_update_snake() -> anyhow::Result<()> {
    let cluster = TestSetup::start_local_test_cluster().await?;
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
        .with_command(Commands::PublishQuilts {
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
    let mut index_html = OpenOptions::new()
        .append(true) // don't truncate, add to the end
        .open(index_html_path)?;
    writeln!(&mut index_html, "<!-- Updated by test -->")?;
    let ws_resources_updated: WSResources =
        serde_json::from_reader(File::open(ws_resources_path.as_path())?)?;

    let site_id = *cluster.last_site_created().await?.id.object_id();
    assert_eq!(ws_resources_updated.object_id.unwrap(), site_id);

    let update_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::UpdateQuilts {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory)
                .with_epoch_count_or_max(EpochCountOrMax::Max)
                .build()?,
            object_id: site_id,
        })
        .build()?;
    site_builder::run(update_args).await?;

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
async fn quilts_update_blob_snake() -> anyhow::Result<()> {
    let cluster = TestSetup::start_local_test_cluster().await?;
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
    let mut index_html = OpenOptions::new()
        .append(true) // don't truncate, add to the end
        .open(index_html_path)?;
    writeln!(index_html, "<!-- Updated by test -->")?;
    let ws_resources_updated: WSResources =
        serde_json::from_reader(File::open(ws_resources_path.as_path())?)?;

    let site_id = *cluster.last_site_created().await?.id.object_id();
    assert_eq!(ws_resources_updated.object_id.unwrap(), site_id);

    let update_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::UpdateQuilts {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory)
                .with_epoch_count_or_max(EpochCountOrMax::Max)
                .build()?,
            object_id: site_id,
        })
        .build()?;
    site_builder::run(update_args).await?;

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
                "index.html should have exactly one more line after update. Original: {original_line_count}, Updated: {updated_line_count}"
            )
        }
    }

    println!("quilts_update_blob_snake completed successfully");

    Ok(())
}

#[tokio::test]
#[ignore]
async fn quilts_update_half_files() -> anyhow::Result<()> {
    const N_FILES_IN_SITE: usize = 100;

    let cluster = TestSetup::start_local_test_cluster().await?;

    // Create a temporary directory for our test site
    let temp_dir = tempfile::tempdir()?;
    let test_site_dir = temp_dir.path().to_owned();

    println!("Creating {N_FILES_IN_SITE} files for the test site...");

    // Step 1: Create many simple HTML files
    fs::create_dir_all(&test_site_dir)?;
    for i in 0..N_FILES_IN_SITE {
        let file_path = test_site_dir.join(format!("{i}.html"));
        let mut file = File::create(file_path)?;
        writeln!(file, "<html><body><h1>Page {i}</h1></body></html>")?;
    }

    println!("Publishing initial site with {N_FILES_IN_SITE} files...");

    // Step 2: Publish the initial site
    let publish_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::PublishQuilts {
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
        let file_path = test_site_dir.join(format!("{i}.html"));
        let content = fs::read_to_string(&file_path)?;
        let updated_content = content.replace(&format!("Page {i}"), &format!("UPDATED Page {i}"));
        fs::write(&file_path, updated_content)?;
    }

    // Step 4: Update the site using the Update command
    println!("Updating site with modified files...");
    let update_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::UpdateQuilts {
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

        // Extract file number from path (e.g., "/42.html" -> 42)
        let file_number = resource
            .path
            .strip_prefix('/')
            .and_then(|p| p.strip_suffix(".html"))
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

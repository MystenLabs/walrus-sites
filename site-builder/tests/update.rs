// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{
    fs::{self, File},
    io::Write,
    str::FromStr,
};

use fastcrypto::hash::{HashFunction, Sha256};
use move_core_types::{language_storage::StructTag, u256::U256};
use site_builder::args::{Commands, EpochCountOrMax};
use sui_sdk::rpc_types::SuiObjectResponseQuery;
use sui_types::Identifier;
use walrus_sdk::core::BlobId;

#[allow(dead_code)]
mod localnode;
use localnode::{
    args_builder::{ArgsBuilder, PublishOptionsBuilder},
    TestSetup,
};

#[tokio::test]
#[ignore]
async fn update_half_files() -> anyhow::Result<()> {
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
        let file_path = test_site_dir.join(format!("{i}.html"));
        let content = fs::read_to_string(&file_path)?;
        let updated_content = content.replace(&format!("Page {i}"), &format!("UPDATED Page {i}"));
        fs::write(&file_path, updated_content)?;
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
            watch: false,
            force: false,
            check_extend: false,
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
        let data = cluster.read_blob(&BlobId(resource.blob_id.0)).await?;
        let mut hash_function = Sha256::default();
        hash_function.update(&data);
        let resource_hash: [u8; 32] = hash_function.finalize().digest;
        assert_eq!(resource.blob_hash, U256::from_le_bytes(&resource_hash));

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

    println!("Update test with {N_FILES_IN_SITE} files completed successfully!",);

    Ok(())
}

#[tokio::test]
#[ignore]
async fn update_with_file_rename() -> anyhow::Result<()> {
    const N_FILES: usize = 10;

    let mut cluster = TestSetup::start_local_test_cluster().await?;

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
                .with_epoch_count_or_max(EpochCountOrMax::Max)
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

    // Step 3: Rename one file (file_0.html -> renamed_file.html)
    println!("Renaming file_0.html to renamed_file.html...");
    let old_path = test_site_dir.join("file_0.html");
    let new_path = test_site_dir.join("renamed_file.html");
    fs::rename(&old_path, &new_path)?;

    // Step 4: Update the site with the renamed file
    println!("Updating site with renamed file...");
    let update_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Update {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(test_site_dir)
                .with_epoch_count_or_max(EpochCountOrMax::Max)
                .build()?,
            object_id: site_object_id,
            watch: false,
            force: false,
            check_extend: false,
        })
        .with_gas_budget(10_000_000_000)
        .build()?;

    site_builder::run(update_args).await?;

    println!("Successfully updated site with renamed file");

    // Step 5: Use SuiClient to verify objects under the wallet address
    let wallet_address = cluster.wallet.inner.active_address()?;
    println!("Verifying objects owned by address: {wallet_address}");

    // Get all objects owned by the wallet address
    let owned_blobs = cluster
        .client
        .read_api()
        .get_owned_objects(
            wallet_address,
            Some(SuiObjectResponseQuery::new_with_filter(
                sui_sdk::rpc_types::SuiObjectDataFilter::StructType(StructTag {
                    address: cluster.cluster_state.system_context.walrus_pkg_id.into(),
                    module: Identifier::from_str("blob")?,
                    name: Identifier::from_str("Blob")?,
                    type_params: vec![],
                }),
            )),
            None,
            None,
        )
        .await?;

    let blob_count = owned_blobs.data.len();

    assert_eq!(
        blob_count, N_FILES,
        "Expected {N_FILES} blob objects, found {}",
        blob_count
    );

    // Step 6: Verify the resources through site_resources method
    let updated_resources = cluster.site_resources(site_object_id).await?;
    assert_eq!(updated_resources.len(), N_FILES);

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

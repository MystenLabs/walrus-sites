// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{fs, fs::File, path::PathBuf};

use fastcrypto::hash::{HashFunction, Sha256};
use move_core_types::u256::U256;
use site_builder::{
    args::{Commands, EpochCountOrMax},
    site_config::WSResources,
};
use walrus_sdk::core::{metadata::QuiltMetadata, BlobId};

#[allow(dead_code)]
mod helpers;
#[allow(dead_code)]
mod localnode;
use helpers::copy_dir;
use localnode::{
    args_builder::{ArgsBuilder, PublishOptionsBuilder},
    TestSetup,
};

// Helper functions for list_directory tests

/// Verify quilt contains expected identifiers and doesn't contain ignored ones.
fn assert_quilt_identifiers(
    quilt_identifiers: &[&str],
    expected_present: &[&str],
    expected_absent: &[&str],
    context: &str,
) {
    for identifier in expected_present {
        assert!(
            quilt_identifiers.contains(identifier),
            "{context}: Expected identifier {identifier} to be present in quilt metadata",
        );
    }

    for identifier in expected_absent {
        assert!(
            !quilt_identifiers.contains(identifier),
            "{context}: Ignored file identifier {identifier} should NOT be present in quilt metadata",
        );
    }
}

// This test verifies that the site-builder can run the list-directory command
// for the examples/snake directory without publishing it.
// Makes sure that the resources that should be ignored are not included in the
// produced `index.html` files.
#[tokio::test]
#[ignore]
async fn preprocess_the_snake_example_with_list_directory_no_publish() -> anyhow::Result<()> {
    let cluster = TestSetup::start_local_test_cluster().await?;
    let snake_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("examples")
        .join("snake");

    // Copy the entire snake directory to a temp location to avoid modifying the original
    let temp_dir = tempfile::tempdir()?;
    let directory = temp_dir.path().join("snake");
    copy_dir(&snake_dir, &directory)?;

    let temp_ws_resources = directory.join("ws-resources.json");

    let args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::ListDirectory {
            path: directory.clone(),
            ws_resources: Some(temp_ws_resources),
        })
        .build()?;

    site_builder::run(args).await?;
    let index_content = fs::read_to_string(directory.join("index.html"))?;
    // Make sure that `secret.txt` was not included in the output index.html
    assert!(!index_content.contains("<li><a href=\"/secret.txt\">secret.txt</a></li>"));
    // Make sure that `/private` was not included in the root index.html contents
    assert!(!index_content.contains("<li><a href=\"/private/index.html\">private/</a></li>"));
    // Make sure that no `/private/index.html` file was generated, nor `/private/nested/index.html`
    assert!(!directory.join("private").join("index.html").exists());
    assert!(!directory
        .join("private")
        .join("nested")
        .join("index.html")
        .exists());

    Ok(())
}

#[tokio::test]
#[ignore]
// This test verifies that the site-builder can publish using Quilts
// with the --list-directory option enabled, ensuring that preprocessing
// works correctly with the Quilts publishing flow and that the generated
// index.html is published as a Quilt resource.
async fn publish_quilts_with_list_directory() -> anyhow::Result<()> {
    const SNAKE_FILES_UPLOAD_FILES: usize = 4;
    let cluster = TestSetup::start_local_test_cluster().await?;
    let snake_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("examples")
        .join("snake");

    // Copy the entire snake directory to a temp location to avoid modifying the original
    let temp_dir = tempfile::tempdir()?;
    let directory = temp_dir.path().join("snake");
    copy_dir(&snake_dir, &directory)?;

    let args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Publish {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory.clone())
                .with_list_directory(true)
                .with_epoch_count_or_max(EpochCountOrMax::Max)
                .build()?,
            site_name: None,
        })
        .build()?;
    site_builder::run(args).await?;

    let site = cluster.last_site_created().await?;
    let resources = cluster.site_resources(*site.id.object_id()).await?;

    // Should have the same number of resources as regular publish with list_directory
    assert_eq!(resources.len(), SNAKE_FILES_UPLOAD_FILES);

    for resource in &resources {
        let blob_id = BlobId(resource.blob_id.0);
        let patch_id = resource.headers.0.get("x-wal-quilt-patch-internal-id");
        assert!(patch_id.is_some(), "Resource should have a quilt patch ID");
        let patch_id_bytes = hex::FromHex::from_hex(patch_id.unwrap().trim_start_matches("0x"))?;
        let res = cluster
            .read_quilt_patches(&[walrus_sdk::core::QuiltPatchId {
                patch_id_bytes,
                quilt_id: blob_id,
            }])
            .await?;
        assert_eq!(res.len(), 1);

        let mut hash_function = Sha256::default();
        hash_function.update(res[0].data());
        let resource_hash: [u8; 32] = hash_function.finalize().digest;
        assert_eq!(resource.blob_hash, U256::from_le_bytes(&resource_hash));

        // Make sure that the ignored files in ws-resources.json are not published.
        let ignored_paths = [
            "/secret.txt",
            "/private/data.txt",
            "/private/nested/hidden.doc",
        ];
        for ignored_path in &ignored_paths {
            assert_ne!(resource.path, *ignored_path);
        }
    }

    // Verify that the generated index.html exists and was published as a resource
    let index_resource = resources
        .iter()
        .find(|r| r.path == "/index.html")
        .expect("index.html should be published as a resource");

    // Read the index.html content via Quilts (fetch from Walrus)
    let blob_id = BlobId(index_resource.blob_id.0);
    let patch_id = index_resource
        .headers
        .0
        .get("x-wal-quilt-patch-internal-id")
        .unwrap();
    let patch_id_bytes = hex::FromHex::from_hex(patch_id.trim_start_matches("0x"))?;
    let quilt_patches = cluster
        .read_quilt_patches(&[walrus_sdk::core::QuiltPatchId {
            patch_id_bytes,
            quilt_id: blob_id,
        }])
        .await?;

    assert_eq!(quilt_patches.len(), 1);
    let index_content = String::from_utf8(quilt_patches[0].data().to_vec())?;

    // Verify the HTML structure matches the expected format for list-directory
    assert!(index_content.contains(
        r#"<!DOCTYPE html>
<html>
<head>
<title>Directory listing for /</title>
</head>
<body>
<h1>Directory listing for /</h1>
<hr>
<ul>"#
    ));
    assert!(index_content.contains("</ul>"));

    // Verify that the expected files from snake example are listed in the index
    // (these files should not be ignored according to ws-resources.json)
    let expected_files = ["Oi-Regular.ttf", "file.svg", "walrus.svg"];
    for file in &expected_files {
        assert!(
            index_content.contains(&format!("<li><a href=\"/{file}\">{file}</a></li>")),
            "Expected file {file} to be listed in index.html",
        );
    }

    // Verify ignored files are not in the generated index.html
    assert!(!index_content.contains("secret.txt"));
    assert!(!index_content.contains("private/"));
    assert!(!index_content.contains("/private/"));

    Ok(())
}

#[tokio::test]
#[ignore]
// This test verifies that DeployQuilts with --list-directory correctly handles
// changing ignore patterns, ensuring that newly ignored files are removed from
// the quilt and previously ignored files are added.
async fn deploy_quilts_with_list_directory_updates_ignored_files() -> anyhow::Result<()> {
    // With --list-directory: 3 HTML files + 1 generated index.html = 4 resources
    const EXPECTED_RESOURCES_PER_DEPLOY: usize = 4;
    let mut cluster = TestSetup::start_local_test_cluster().await?;

    // Step 1: Create test site with 5 files (file_0.html through file_4.html)
    let temp_dir = helpers::create_test_site(5)?;
    let directory = temp_dir.path();

    // Step 2: Create ws-resources.json with initial ignore patterns
    // Testing different glob formats:
    // - "/file_2.html" (leading slash format)
    // - "**/file_3.html" (globstar format)
    // - "file_4.html" (TODO: .gitignore pattern matching not yet supported, commented out)
    let ws_resources_path = directory.join("ws-resources.json");
    let ws_resources = WSResources {
        ignore: Some(vec![
            "/file_2.html".to_string(),
            "**/file_3.html".to_string(),
            // "file_4.html".to_string(),  // TODO: implement .gitignore pattern matching
        ]),
        ..Default::default()
    };
    serde_json::to_writer_pretty(File::create(&ws_resources_path)?, &ws_resources)?;

    // Step 3: First deploy with --list-directory
    let args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Deploy {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory.to_path_buf())
                .with_list_directory(true)
                .with_epoch_count_or_max(EpochCountOrMax::Max)
                .build()?,
            site_name: None,
            object_id: None,
        })
        .build()?;
    site_builder::run(args).await?;

    let site = cluster.last_site_created().await?;
    let resources_first = cluster.site_resources(*site.id.object_id()).await?;

    // Verify 4 resources published (file_0.html, file_1.html, file_4.html, index.html)
    assert_eq!(resources_first.len(), EXPECTED_RESOURCES_PER_DEPLOY);
    let paths_first: Vec<&str> = resources_first.iter().map(|r| r.path.as_str()).collect();
    assert!(paths_first.contains(&"/file_0.html"));
    assert!(paths_first.contains(&"/file_1.html"));
    assert!(paths_first.contains(&"/file_4.html")); // TODO (.gitignore)
    assert!(
        paths_first.contains(&"/index.html"),
        "Generated index.html should be present"
    );
    assert!(!paths_first.contains(&"/file_2.html"));
    assert!(!paths_first.contains(&"/file_3.html"));

    // Verify quilt metadata for first deploy
    let quilt_resource_first = resources_first
        .iter()
        .find(|r| r.headers.0.contains_key("x-wal-quilt-patch-internal-id"))
        .expect("Should have at least one quilt resource");

    let quilt_blob_id_first = BlobId(quilt_resource_first.blob_id.0);
    let QuiltMetadata::V1(metadata_v1) = cluster.read_quilt_metadata(&quilt_blob_id_first).await?;
    let quilt_identifiers_first: Vec<_> = metadata_v1
        .index
        .quilt_patches
        .iter()
        .map(|p| p.identifier.as_str())
        .collect();

    // Verify count
    assert_eq!(
        quilt_identifiers_first.len(),
        EXPECTED_RESOURCES_PER_DEPLOY,
        "First deploy: Quilt should contain exactly {EXPECTED_RESOURCES_PER_DEPLOY} patches after ignoring file_2 and file_3",
    );

    let expected_present_first = [
        "/file_0.html",
        "/file_1.html",
        "/file_4.html",
        "/index.html",
    ];

    let expected_ignored_first = ["/file_2.html", "/file_3.html"];

    // Verify that expected files are present and ignored files are absent
    assert_quilt_identifiers(
        &quilt_identifiers_first,
        &expected_present_first,
        &expected_ignored_first,
        "First deploy",
    );

    // Step 4: Update ignore patterns (now ignore file_0.html and file_1.html instead)
    // Using different glob patterns:
    // - "/file_0.html" (leading slash format)
    // - "**/file_1.html" (globstar format)
    let mut ws_resources: WSResources = serde_json::from_reader(File::open(&ws_resources_path)?)?;
    ws_resources.ignore = Some(vec![
        "/file_0.html".to_string(),
        "**/file_1.html".to_string(),
    ]);
    serde_json::to_writer_pretty(File::create(&ws_resources_path)?, &ws_resources)?;

    // Step 5: Second deploy with changed ignore patterns
    let args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Deploy {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory.to_path_buf())
                .with_list_directory(true)
                .with_epoch_count_or_max(EpochCountOrMax::Max)
                .build()?,
            site_name: None,
            object_id: None,
        })
        .build()?;
    site_builder::run(args).await?;

    let site = cluster.last_site_created().await?;
    let resources_second = cluster.site_resources(*site.id.object_id()).await?;

    // Step 6: Verification - Check site resources
    // Verify 4 resources published (file_2.html, file_3.html, file_4.html, index.html)
    // This verifies the on-chain site resources are correct
    assert_eq!(resources_second.len(), EXPECTED_RESOURCES_PER_DEPLOY);
    let paths_second: Vec<&str> = resources_second.iter().map(|r| r.path.as_str()).collect();
    assert!(paths_second.contains(&"/file_2.html"));
    assert!(paths_second.contains(&"/file_3.html"));
    assert!(paths_second.contains(&"/file_4.html"));
    assert!(paths_second.contains(&"/index.html"));
    assert!(!paths_second.contains(&"/file_0.html"));
    assert!(!paths_second.contains(&"/file_1.html"));

    // Step 7: Verification - Parse quilt metadata to verify the quilt structure
    // This verifies that the Walrus quilt blob contains the correct files
    // Find a resource with quilt patch ID to get the quilt blob ID
    let quilt_resource = resources_second
        .iter()
        .find(|r| r.headers.0.contains_key("x-wal-quilt-patch-internal-id"))
        .expect("Should have at least one quilt resource");

    let quilt_blob_id = BlobId(quilt_resource.blob_id.0);
    let QuiltMetadata::V1(metadata_v1) = cluster.read_quilt_metadata(&quilt_blob_id).await?;

    // Step 7: Verification Method 2 - Verify quilt metadata contains correct files
    // Extract identifiers from the quilt metadata
    let quilt_identifiers: Vec<_> = metadata_v1
        .index
        .quilt_patches
        .iter()
        .map(|p| p.identifier.as_str())
        .collect();

    // Verify count
    assert_eq!(
        quilt_identifiers.len(),
        EXPECTED_RESOURCES_PER_DEPLOY,
        "Quilt should contain exactly {EXPECTED_RESOURCES_PER_DEPLOY} patches after ignoring file_0 and file_1",
    );

    let expected_present = [
        "/file_2.html",
        "/file_3.html",
        "/file_4.html",
        "/index.html",
    ];

    let expected_ignored = ["/file_0.html", "/file_1.html"];

    // Verify that expected files are present and ignored files are absent
    assert_quilt_identifiers(
        &quilt_identifiers,
        &expected_present,
        &expected_ignored,
        "Second deploy",
    );

    // Step 8: Verify that the old quilt blob from first deploy was deleted
    let wallet_address = cluster.wallet_active_address()?;
    let owned_blobs = cluster.get_owned_blobs(wallet_address).await?;
    let owned_blob_ids: Vec<BlobId> = owned_blobs.iter().map(|b| b.blob_id).collect();

    assert!(
        !owned_blob_ids.contains(&quilt_blob_id_first),
        "Old quilt blob from first deploy (ID: {quilt_blob_id_first:?}) should have been deleted after second deploy",
    );

    Ok(())
}

#[tokio::test]
#[ignore]
// This test verifies that DeployQuilts with --list-directory correctly handles ws-resources.json:
// 1. Uses the siteName from the argument ws-resources.json (not from the directory)
// 2. Includes the ws-resources.json file itself in the deployed resources and index.html
async fn deploy_quilts_with_list_directory_handles_ws_resources() -> anyhow::Result<()> {
    let cluster = TestSetup::start_local_test_cluster().await?;

    // Step 1: Create test site with 2 files
    let temp_dir = helpers::create_test_site(2)?;
    let directory = temp_dir.path();

    // Step 2: Create ws-resources.json IN the site directory with siteName "site-in-directory"
    let ws_resources_in_directory = directory.join("ws-resources.json");
    let ws_resources_dir_content = WSResources {
        site_name: Some("site-in-directory".to_string()),
        ..Default::default()
    };
    serde_json::to_writer_pretty(
        File::create(&ws_resources_in_directory)?,
        &ws_resources_dir_content,
    )?;

    // Step 3: Create ws-resources-override.json IN THE SAME directory with siteName "site-from-argument"
    let ws_resources_argument_path = directory.join("ws-resources-override.json");
    let ws_resources_arg_content = WSResources {
        site_name: Some("site-from-argument".to_string()),
        ..Default::default()
    };
    serde_json::to_writer_pretty(
        File::create(&ws_resources_argument_path)?,
        &ws_resources_arg_content,
    )?;

    // Step 4: Deploy with --list-directory, passing the external ws-resources.json
    let args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Deploy {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory.to_path_buf())
                .with_list_directory(true)
                .with_ws_resources(Some(ws_resources_argument_path.clone()))
                .with_epoch_count_or_max(EpochCountOrMax::Max)
                .build()?,
            site_name: None,
            object_id: None,
        })
        .build()?;
    site_builder::run(args).await?;

    let site = cluster.last_site_created().await?;
    let resources = cluster.site_resources(*site.id.object_id()).await?;

    // Step 5: Verify the site name matches the argument ws-resources.json
    // Read the updated ws-resources.json from the argument location
    let updated_ws_resources: WSResources =
        serde_json::from_reader(File::open(&ws_resources_argument_path)?)?;

    assert_eq!(
        updated_ws_resources.site_name.as_deref(),
        Some("site-from-argument"),
        "Site name should match the ws-resources.json passed as argument, not the one in the directory"
    );

    // Verify that ws-resources-override.json was updated with the object_id
    assert!(
        updated_ws_resources.object_id.is_some(),
        "ws-resources-override.json should have been updated with the site object_id"
    );
    assert_eq!(
        updated_ws_resources.object_id.unwrap(),
        *site.id.object_id(),
        "ws-resources-override.json should contain the correct site object_id"
    );

    // Verify that ws-resources.json in the directory was NOT updated with object_id
    let ws_resources_in_dir: WSResources =
        serde_json::from_reader(File::open(&ws_resources_in_directory)?)?;
    assert!(
        ws_resources_in_dir.object_id.is_none(),
        "ws-resources.json in the site directory should NOT have been updated with object_id"
    );
    assert_eq!(
        ws_resources_in_dir.site_name.as_deref(),
        Some("site-in-directory"),
        "ws-resources.json in directory should still have its original site name"
    );

    // Step 6: Verify both ws-resources files are included in the deployed resources
    let paths: Vec<&str> = resources.iter().map(|r| r.path.as_str()).collect();

    // Site resources include the ws-resources.json not used as ws-resources
    assert!(
        paths.contains(&"/ws-resources.json"),
        "ws-resources.json from site directory should be included in deployed resources"
    );

    // Site resources exclude the ws-resources-override.json passed as ws-resources
    assert!(
        !paths.contains(&"/ws-resources-override.json"),
        "ws-resources-override.json from site directory should not be included in deployed resources"
    );

    // Step 7: Verify both ws-resources files are listed in the generated index.html
    let index_resource = resources
        .iter()
        .find(|r| r.path == "/index.html")
        .expect("index.html should be present");

    let index_content_bytes =
        helpers::verify_resource_and_get_content(&cluster, index_resource).await?;
    let _index_content = String::from_utf8(index_content_bytes)?;

    // TODO(fix): #SEW-462 This assertion is expected to fail - ws-resources.json should be listed
    // but currently isn't
    // assert!(
    //     index_content.contains("ws-resources.json"),
    //     "ws-resources.json should be listed in the generated index.html"
    // );

    // TODO(fix): #SEW-462 This assertion is expected to fail - ws-resources-override.json should be
    // listed but currently isn't
    // assert!(
    //     !index_content.contains("ws-resources-override.json"),
    //     "ws-resources-override.json should not be listed in the generated index.html"
    // );

    // Step 8: Verify quilt metadata contains ws-resources.json and NOT ws-resources-override.json

    let quilt_resource = resources
        .iter()
        .find(|r| r.headers.0.contains_key("x-wal-quilt-patch-internal-id"))
        .expect("Should have at least one quilt resource");

    let quilt_blob_id = BlobId(quilt_resource.blob_id.0);
    let QuiltMetadata::V1(metadata_v1) = cluster.read_quilt_metadata(&quilt_blob_id).await?;
    let quilt_identifiers: Vec<_> = metadata_v1
        .index
        .quilt_patches
        .iter()
        .map(|p| p.identifier.as_str())
        .collect();

    let ws_resources_json_identifier = "/ws-resources.json";
    let ws_resources_override_identifier = "/ws-resources-override.json";

    // Walrus store includes the ws-resources.json in site-root, not used as ws-resources
    assert!(
        quilt_identifiers.contains(&ws_resources_json_identifier),
        "ws-resources.json should be present in the quilt metadata"
    );

    // Walrus store excludes the ws-resources-override.json, passed as ws-resources.
    assert!(
        !quilt_identifiers.contains(&ws_resources_override_identifier),
        "ws-resources-override.json should NOT be present in the quilt metadata (it's the config file, not site content)"
    );

    Ok(())
}

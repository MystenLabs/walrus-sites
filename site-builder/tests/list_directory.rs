// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{fs, path::PathBuf};

use fastcrypto::hash::{HashFunction, Sha256};
use move_core_types::u256::U256;
use site_builder::args::{Commands, EpochCountOrMax};
use walrus_sdk::core::BlobId;

#[allow(dead_code)]
mod helpers;
#[allow(dead_code)]
mod localnode;
use helpers::copy_dir;
use localnode::{
    args_builder::{ArgsBuilder, PublishOptionsBuilder},
    TestSetup,
};

#[tokio::test]
#[ignore]
// This test verifies that the site-builder can publish the example snake
// with the --list-directory command and assert that the ignored resources
// are published on-chain.
async fn publish_snake_with_list_directory() -> anyhow::Result<()> {
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
                .with_directory(directory)
                .with_list_directory(true)
                .with_epoch_count_or_max(EpochCountOrMax::Max)
                .build()?,
            site_name: None,
        })
        .build()?;
    site_builder::run(args).await?;

    let site = cluster.last_site_created().await?;
    let resources = cluster.site_resources(*site.id.object_id()).await?;

    assert_eq!(resources.len(), SNAKE_FILES_UPLOAD_FILES);

    for resource in resources {
        let data = cluster.read_blob(&BlobId(resource.blob_id.0)).await?;
        let mut hash_function = Sha256::default();
        hash_function.update(&data);
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
        // Make sure that the produced index.html from the list-directory command does
        // not include paths to the ignored files of ws-resources.json
    }

    Ok(())
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

#[cfg(feature = "quilts-experimental")]
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
        .with_command(Commands::PublishQuilts {
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

// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf}, time::Instant,
};

use fastcrypto::hash::{HashFunction, Sha256};
use hex::FromHex;
use move_core_types::u256::U256;
use site_builder::args::{Commands, EpochCountOrMax};

#[allow(dead_code)]
mod localnode;
use localnode::{
    args_builder::{ArgsBuilder, PublishOptionsBuilder},
    TestSetup,
};
use walrus_sdk::core::{BlobId, QuiltPatchId};

#[tokio::test]
async fn publish_snake() -> anyhow::Result<()> {
    const SNAKE_FILES_UPLOAD_FILES: usize = 4;
    let cluster = TestSetup::start_local_test_cluster().await?;
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("examples")
        .join("snake");

    let og_ws_resources = directory.join("ws-resources.json");
    // Create a temp file copy so the original doesn't get mutated during the test.
    let temp_dir = tempfile::tempdir()?;
    let temp_ws_resources = temp_dir.path().join("ws-resources.json");
    fs::copy(&og_ws_resources, &temp_ws_resources)?;

    let args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Publish {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory)
                .with_ws_resources(Some(temp_ws_resources))
                .with_epoch_count_or_max(EpochCountOrMax::Max)
                .build()?,
            site_name: None,
        })
        .build()?;
    site_builder::run(args).await?;

    let site = cluster.last_site_created().await?;
    let resources = cluster.site_resources(*site.id.object_id()).await?;

    assert_eq!(resources.len(), SNAKE_FILES_UPLOAD_FILES + 1); // +1 because we use a temp
                                                               // ws-resources
    for resource in resources {
        let data = cluster.read_blob(&BlobId(resource.blob_id.0)).await?;
        let mut hash_function = Sha256::default();
        hash_function.update(&data);
        let resource_hash: [u8; 32] = hash_function.finalize().digest;
        assert_eq!(resource.blob_hash, U256::from_le_bytes(&resource_hash));
    }

    Ok(())
}

#[tokio::test]
async fn quilts_publish_snake() -> anyhow::Result<()> {
    const SNAKE_FILES_UPLOAD_FILES: usize = 4;

    let cluster = TestSetup::start_local_test_cluster().await?;
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("examples")
        .join("snake");

    let og_ws_resources = directory.join("ws-resources.json");
    // Create a temp file copy so the original doesn't get mutated during the test.
    let temp_dir = tempfile::tempdir()?;
    let temp_ws_resources = temp_dir.path().join("ws-resources.json");
    fs::copy(&og_ws_resources, &temp_ws_resources)?;

    let args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::PublishQuilts {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory)
                .with_ws_resources(Some(temp_ws_resources))
                .with_epoch_count_or_max(EpochCountOrMax::Max)
                .build()?,
            site_name: None,
        })
        .build()?;
    site_builder::run(args).await?;

    let site = cluster.last_site_created().await?;
    let resources = cluster.site_resources(*site.id.object_id()).await?;

    assert_eq!(resources.len(), SNAKE_FILES_UPLOAD_FILES + 1); // +1 because we use a temp
                                                               // ws-resources
    for resource in resources {
        let blob_id = BlobId(resource.blob_id.0);
        let patch_id = resource.headers.0.get("x-wal-quilt-patch-internal-id");
        assert!(patch_id.is_some());
        let patch_id_bytes =
            Vec::from_hex(patch_id.unwrap().trim_start_matches("0x")).expect("Invalid hex");
        let res = cluster
            .read_quilt_patches(&[QuiltPatchId {
                patch_id_bytes,
                quilt_id: blob_id,
            }])
            .await?;
        assert_eq!(res.len(), 1);

        let mut hash_function = Sha256::default();
        hash_function.update(res[0].data());
        let resource_hash: [u8; 32] = hash_function.finalize().digest;
        assert_eq!(resource.blob_hash, U256::from_le_bytes(&resource_hash));
    }

    Ok(())
}

#[tokio::test]
#[ignore]
async fn publish_quilts_lots_of_files() -> anyhow::Result<()> {
    const N_FILES_IN_SITE: usize = 900;

    let cluster = TestSetup::start_local_test_cluster().await?;

    let temp_dir = tempfile::tempdir()?;
    // Generate 100 files: 1.html, 2.html, ..., 100.html
    (0..N_FILES_IN_SITE).try_for_each(|i| {
        let file_path = temp_dir.path().join(format!("{i}.htlm"));
        let mut file = File::create(file_path)?;
        writeln!(file, "<html><body><h1>File {i}</h1></body></html>")?;
        Ok::<(), anyhow::Error>(())
    })?;

    let publish_start = Instant::now();
    let args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::PublishQuilts {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(temp_dir.path().to_owned())
                .with_epoch_count_or_max(EpochCountOrMax::Max)
                .build()?,
            site_name: None,
        })
        .with_gas_budget(50_000_000_000)
        .build()?;
    site_builder::run(args).await?;
    println!("Publishing took {:#?}", publish_start.elapsed());

    let site = cluster.last_site_created().await?;
    let resources = cluster.site_resources(*site.id.object_id()).await?;
    assert_eq!(resources.len(), N_FILES_IN_SITE);

    // This could be a bit optimized by fetching the whole blobs maybe. (for TestCluster ~= /8 less
    // get-quilt calls)
    let fetching_start = Instant::now();
    for resource in resources {
        let blob_id = BlobId(resource.blob_id.0);
        let patch_id = resource.headers.0.get("x-wal-quilt-patch-internal-id");
        assert!(patch_id.is_some());
        let patch_id_bytes =
            Vec::from_hex(patch_id.unwrap().trim_start_matches("0x")).expect("Invalid hex");

        let index: usize = Path::new(resource.path.as_str())
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .parse()?;
        let mut res = cluster
            .read_quilt_patches(&[QuiltPatchId {
                patch_id_bytes,
                quilt_id: blob_id,
            }])
            .await?;
        assert_eq!(res.len(), 1);

        let data = res.remove(0).into_data();
        let text_file_contents = String::from_utf8(data)?;
        assert_eq!(
            text_file_contents,
            format!("<html><body><h1>File {index}</h1></body></html>\n")
        );
    }
    println!("Fetching took {:#?}", fetching_start.elapsed());

    Ok(())
}

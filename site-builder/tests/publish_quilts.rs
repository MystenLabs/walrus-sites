// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

#![cfg(feature = "quilts-experimental")]

use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    time::Instant,
};

use fastcrypto::hash::{HashFunction, Sha256};
use hex::FromHex;
use move_core_types::u256::U256;
use site_builder::{
    args::{Commands, EpochCountOrMax},
    site_config::WSResources,
    types::{HttpHeaders, Routes, VecMap},
};
use walrus_sdk::core::{BlobId, QuiltPatchId};

#[allow(dead_code)]
mod localnode;
use localnode::{
    args_builder::{ArgsBuilder, PublishOptionsBuilder},
    TestSetup,
};

#[tokio::test]
#[ignore]
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
        let file_path = temp_dir.path().join(format!("{i}.html"));
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

#[tokio::test]
#[ignore]
async fn publish_quilts_lots_of_identical_files() -> anyhow::Result<()> {
    let cluster = TestSetup::start_local_test_cluster().await?;

    let n_shards = cluster.cluster_state.walrus_cluster.n_shards;
    let n_files_per_dir =
        (u16::from(walrus_core::encoding::source_symbols_for_n_shards(n_shards).1) as usize - 1)
            * 10; // 10 is arbitrary

    let temp_dir = tempfile::tempdir()?;
    let subdir1 = temp_dir.path().join("subdir1");
    let subdir2 = temp_dir.path().join("subdir2");
    fs::create_dir(&subdir1)?;
    fs::create_dir(&subdir2)?;

    [&subdir1, &subdir2].iter().try_for_each(|subdir| {
        (0..n_files_per_dir).try_for_each(|i| {
            let file_path = subdir.join(format!("{i}.html"));
            let mut file = File::create(file_path)?;
            writeln!(file, "<html><body><h1>File</h1></body></html>")?;
            Ok::<(), anyhow::Error>(())
        })
    })?;

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

    let site = cluster.last_site_created().await?;
    let resources = cluster.site_resources(*site.id.object_id()).await?;
    assert_eq!(resources.len(), n_files_per_dir);

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

#[tokio::test]
#[ignore]
async fn publish_quilts_a_lot_of_headers() -> anyhow::Result<()> {
    const N_FILES_IN_SITE: usize = 10;
    const EXTRA_HEADERS_PER_HTML: usize = 100;

    let cluster = TestSetup::start_local_test_cluster().await?;

    // Create a fake site
    let site_temp_dir = tempfile::tempdir()?;
    // Generate 100 files: 1.html, 2.html, ..., 100.html
    (0..N_FILES_IN_SITE).try_for_each(|i| {
        let file_path = site_temp_dir.path().join(format!("{i}.html"));
        let mut file = File::create(file_path)?;
        writeln!(file, "<html><body><h1>File {i}</h1></body></html>")?;
        Ok::<(), anyhow::Error>(())
    })?;

    // Create a ws-resources with a lot of headers per .html file.
    let headers = BTreeMap::<String, HttpHeaders>::from([(
        "*.html".to_string(),
        HttpHeaders(VecMap(
            (0..EXTRA_HEADERS_PER_HTML)
                .map(|i| {
                    (
                        format!("custom_header_key_{i:02}"),
                        format!("custom_header_value_{i:02}"),
                    )
                })
                .collect(),
        )),
    )]);
    let ws_resources = WSResources {
        headers: Some(headers),
        ..Default::default()
    };

    let ws_temp_dir = tempfile::tempdir()?;
    let temp_ws_resources = ws_temp_dir.path().join("ws-resources.json");
    let file = File::create(&temp_ws_resources)?;
    serde_json::to_writer_pretty(BufWriter::new(file), &ws_resources)?;

    // ws_resources.headers.
    let args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::PublishQuilts {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(site_temp_dir.path().to_owned())
                .with_ws_resources(Some(temp_ws_resources))
                .with_epoch_count_or_max(EpochCountOrMax::Max)
                .build()?,
            site_name: None,
        })
        .with_gas_budget(50_000_000_000)
        .build()?;

    site_builder::run(args).await?;

    let site = cluster.last_site_created().await?;
    let resources = cluster.site_resources(*site.id.object_id()).await?;
    assert_eq!(resources.len(), N_FILES_IN_SITE);

    resources.into_iter().for_each(|r| {
        assert_eq!(
            r.headers.len(),
            EXTRA_HEADERS_PER_HTML + 3 /* content-encoding + content-type + quilt-patch-id */
        )
    });

    Ok(())
}

#[tokio::test]
#[ignore]
async fn publish_quilts_with_many_routes() -> anyhow::Result<()> {
    const N_RESOURCES: usize = 800;
    const N_ROUTES: usize = 400;

    let cluster = TestSetup::start_local_test_cluster().await?;

    // Create 800 HTML files (resources)
    let site_temp_dir = tempfile::tempdir()?;
    (0..N_RESOURCES).try_for_each(|i| {
        let file_path = site_temp_dir.path().join(format!("page_{i:03}.html"));
        let mut file = File::create(file_path)?;
        writeln!(file, "<html><body><h1>Resource {i}</h1></body></html>")?;
        Ok::<(), anyhow::Error>(())
    })?;

    // Create 400 routes: redirect resources 400-799 to resources 0-399
    // This creates routes that point the 2nd half of resources to the 1st half
    let routes = Routes(VecMap(
        (0..N_ROUTES)
            .map(|i| {
                // Route from page_4xx.html to page_0xx.html
                let source_resource = format!("/page_{:03}.html", i + N_ROUTES); // 400-799
                let target_resource = format!("/page_{i:03}.html"); // 0-399
                (source_resource, target_resource)
            })
            .collect(),
    ));

    let ws_resources = WSResources {
        routes: Some(routes),
        ..Default::default()
    };

    let ws_temp_dir = tempfile::tempdir()?;
    let temp_ws_resources = ws_temp_dir.path().join("ws-resources.json");
    let file = File::create(&temp_ws_resources)?;
    serde_json::to_writer_pretty(BufWriter::new(file), &ws_resources)?;

    let args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::PublishQuilts {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(site_temp_dir.path().to_owned())
                .with_ws_resources(Some(temp_ws_resources))
                .with_epoch_count_or_max(EpochCountOrMax::Max)
                .build()?,
            site_name: None,
        })
        .with_gas_budget(100_000_000_000) // Higher gas budget for the many move calls
        .build()?;

    // This should succeed despite having 800 resources + 400 routes because the site builder
    // should handle the move call limits by splitting operations across multiple PTBs
    site_builder::run(args).await.inspect_err(|e| {
        println!("error running site-builder: {e}");
        // Apply this for debugging with sui-explorers
        // cluster.wait_for_user_input().await?;
    })?;

    let site = cluster.last_site_created().await?;
    let resources = cluster.site_resources(*site.id.object_id()).await?;
    assert_eq!(resources.len(), N_RESOURCES); // All 800 resources should be created

    // Verify that the site was created successfully with many resources and routes
    // The routes redirect the 2nd half of resources to the 1st half
    println!("Successfully created site with {N_RESOURCES} resources and {N_ROUTES} routes");

    Ok(())
}

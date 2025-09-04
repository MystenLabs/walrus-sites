// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{fs, path::PathBuf};

use site_builder::args::{Commands, EpochCountOrMax};

#[allow(dead_code)]
mod localnode;
use localnode::{
    args_builder::{ArgsBuilder, PublishOptionsBuilder},
    TestSetup,
};

#[tokio::test]
async fn publish_snake() -> anyhow::Result<()> {
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
    println!("site_id: {site:#?}");
    println!(
        "fields: {:#?}",
        cluster.site_resources(*site.id.object_id()).await?
    );

    Ok(())
}

#[tokio::test]
async fn quilts_publish_snake() -> anyhow::Result<()> {
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
        .with_command(Commands::QuiltsPublish {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory)
                .with_ws_resources(Some(temp_ws_resources))
                .with_epoch_count_or_max(EpochCountOrMax::Max)
                .build()?,
            site_name: None,
        })
        .build()?;
    site_builder::run(args).await?;

    Ok(())
}

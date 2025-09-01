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

// Important: For tests to pass, the system they are running on need to have walrus installed.
#[tokio::test]
async fn publish_snake() -> anyhow::Result<()> {
    let mut cluster = TestSetup::start_local_test_cluster().await?;
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("examples")
        .join("snake");
    println!("active_address: {}", cluster.wallet.inner.active_address()?);
    println!("FN URL: {}", cluster.wallet.inner.get_rpc_url()?);

    let og_ws_resources = directory.join("ws-resources.json");
    // Create a temp file copy so the original doesn't get mutated during the test.
    let temp_dir = tempfile::tempdir()?;
    let temp_ws_resources = temp_dir.path().join("ws-resources.json");
    fs::copy(&og_ws_resources, &temp_ws_resources)?;

    let args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config.inner.1))
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

    // use tokio::io::{self, AsyncBufReadExt, BufReader};
    // println!("Press Enter to continue...");
    // BufReader::new(io::stdin())
    //     .read_line(&mut String::new())
    //     .await
    //     .unwrap();
    // println!("Key pressed!");
    Ok(())
}

// Important: For tests to pass, the system they are running on need to have walrus installed.
#[tokio::test]
async fn quilts_publish_snake() -> anyhow::Result<()> {
    let mut cluster = TestSetup::start_local_test_cluster().await?;
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("examples")
        .join("snake");
    println!("active_address: {}", cluster.wallet.inner.active_address()?);
    println!("FN URL: {}", cluster.wallet.inner.get_rpc_url()?);

    let og_ws_resources = directory.join("ws-resources.json");
    // Create a temp file copy so the original doesn't get mutated during the test.
    let temp_dir = tempfile::tempdir()?;
    let temp_ws_resources = temp_dir.path().join("ws-resources.json");
    fs::copy(&og_ws_resources, &temp_ws_resources)?;

    let args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config.inner.1))
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

    // use tokio::io::{self, AsyncBufReadExt, BufReader};
    // println!("Press Enter to continue...");
    // BufReader::new(io::stdin())
    //     .read_line(&mut String::new())
    //     .await
    //     .unwrap();
    // println!("Key pressed!");
    Ok(())
}

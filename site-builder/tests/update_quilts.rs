// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

#![cfg(feature = "quilts-experimental")]

use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::PathBuf,
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

mod helpers;

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

    // Update a resource
    let index_html_path = temp_dir.path().join("index.html");
    let mut index_html = OpenOptions::new()
        .append(true) // don’t truncate, add to the end
        .open(index_html_path)?;
    writeln!(index_html)?;
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

    Ok(())
}

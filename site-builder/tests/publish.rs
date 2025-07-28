// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;

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
    let cluster = TestSetup::start_local_test_cluster().await?;
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("examples")
        .join("snake");
    let ws_resources = directory.join("ws-resources.json");

    let args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config.inner.1))
        .with_command(Commands::Publish {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory)
                .with_ws_resources(Some(ws_resources))
                .with_epoch_count_or_max(EpochCountOrMax::Max)
                .build()?,
            site_name: None,
        })
        .build()?;
    site_builder::run(args).await?;

    Ok(())
}

#[tokio::test]
async fn json_publish_snake() -> anyhow::Result<()> {
    let cluster = TestSetup::start_local_test_cluster().await?;
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("examples")
        .join("snake");

    let args = ArgsBuilder::default()
        .with_command(Commands::Json {
            command_string: Some(format!(
                r#"{{
            "config":"{}",
            "command":{{
                "publish":{{
                    "directory":"{}",
                    "epochs":1
                }}
            }}
        }}"#,
                cluster.sites_config.inner.1.to_string_lossy(),
                directory.to_string_lossy()
            )),
        })
        .build()?;
    site_builder::run(args).await?;

    Ok(())
}

// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;

use site_builder::args::{ArgsInner, Commands, EpochCountOrMax, GeneralArgs};

#[allow(dead_code)]
mod localnode;
use localnode::{PublishOptionsBuilder, TestSetup};

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

    let args = ArgsInner {
        config: Some(cluster.sites_config.inner.1),
        context: None,
        general: GeneralArgs::default(),
        command: Commands::Publish {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory)
                .with_ws_resources(Some(ws_resources))
                .with_epoch_count_or_max(EpochCountOrMax::Max)
                .build()?,
            site_name: None,
        },
    };
    site_builder::run(args).await?;

    Ok(())
}

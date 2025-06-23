// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

mod localnode;
use std::{num::NonZeroU32, path::PathBuf};

use localnode::TestSetup;
use site_builder::args::{
    default,
    Commands,
    EpochArg,
    EpochCountOrMax,
    GeneralArgs,
    PublishOptions,
    WalrusStoreOptions,
};

// Important: For tests to pass, the system they are running on need to have walrus installed.
#[tokio::test]
async fn publish_snake() -> anyhow::Result<()> {
    let cluster = TestSetup::new().await?;
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("examples")
        .join("snake");
    let ws_resources = directory.join("ws-resources.json");

    site_builder::run(
        Some(cluster.sites_config.inner.1),
        None,
        GeneralArgs::default(),
        Commands::Publish {
            publish_options: PublishOptions {
                directory,
                list_directory: false,
                max_concurrent: None,
                max_parallel_stores: default::max_parallel_stores(),
                walrus_options: WalrusStoreOptions {
                    ws_resources: Some(ws_resources),
                    epoch_arg: EpochArg {
                        epochs: Some(EpochCountOrMax::Epochs(NonZeroU32::new(1).unwrap())),
                        earliest_expiry_time: None,
                        end_epoch: None,
                    },
                    permanent: false,
                    dry_run: false,
                },
            },
            site_name: None,
        },
    )
    .await?;

    Ok(())
}

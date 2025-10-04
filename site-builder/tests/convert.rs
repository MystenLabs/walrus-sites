// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use site_builder::args::Commands;
use sui_types::base_types::ObjectID;

#[allow(dead_code)]
mod localnode;
use localnode::{args_builder::ArgsBuilder, TestSetup};

// Important: For tests to pass, the system they are running on need to have walrus installed.
#[tokio::test]
#[ignore]
async fn converts_random_site_id() -> anyhow::Result<()> {
    let cluster = TestSetup::start_local_test_cluster().await?;

    let args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_path_buf()))
        .with_command(Commands::Convert {
            object_id: ObjectID::random(),
        })
        .build()?;
    site_builder::run(args).await?;
    Ok(())
}

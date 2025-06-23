// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use site_builder::args::GeneralArgs;
use sui_types::base_types::ObjectID;

#[allow(dead_code)]
mod localnode;
use localnode::TestSetup;

// Important: For tests to pass, the system they are running on need to have walrus installed.
#[tokio::test]
async fn converts_random_site_id() -> anyhow::Result<()> {
    let cluster = TestSetup::start_local_test_cluster().await?;

    site_builder::run(
        Some(cluster.sites_config.inner.1),
        None,
        GeneralArgs::default(),
        site_builder::args::Commands::Convert {
            object_id: ObjectID::random(),
        },
    )
    .await?;
    Ok(())
}

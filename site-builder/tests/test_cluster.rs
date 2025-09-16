// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

#[allow(dead_code)]
mod localnode;
use localnode::{TestSetup, WalrusSitesClusterState};

// Running this in `opt-level = 0` mode can fail with:
// ```
// thread 'localnode::test_cluster_builder' has overflowed its stack
// fatal runtime error: stack overflow
// ```
#[tokio::test]
#[ignore]
async fn start_walrus_sites_cluster() -> anyhow::Result<()> {
    let TestSetup {
        cluster_state:
            WalrusSitesClusterState {
                mut walrus_sites_publisher,
                ..
            },
        walrus_sites_package_id,
        sites_config,
        ..
    } = TestSetup::start_local_test_cluster().await?;
    println!(
        r#"Published walrus_sites
- at {walrus_sites_package_id}
- from the address {} which is generated during Sui Cluster initialization.
- Sites config:
{}"#,
        walrus_sites_publisher.inner.active_address()?,
        serde_yaml::to_string(&sites_config.inner.0)?
    );
    Ok(())
}

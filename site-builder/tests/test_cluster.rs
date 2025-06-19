// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

mod localnode;
use localnode::{WalrusSitesClusterState, WalrusSitesPublisher};

// Running this in `opt-level = 0` mode can fail with:
// ```
// thread 'localnode::test_cluster_builder' has overflowed its stack
// fatal runtime error: stack overflow
// ```
#[tokio::test]
async fn start_walrus_sites_cluster() -> anyhow::Result<()> {
    let WalrusSitesClusterState {
        walrus_sites_publisher: WalrusSitesPublisher::FromSuiClusterHandle(publisher),
        walrus_sites_package_id,
        ..
    } = WalrusSitesClusterState::new().await?;
    println!(
        r#"Published walrus_sites
- at {walrus_sites_package_id}
- from the address {publisher} which is generated during Sui Cluster initialization."#
    );
    Ok(())
}

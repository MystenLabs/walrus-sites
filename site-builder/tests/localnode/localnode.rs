// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0
//
use anyhow::Result;

use walrus_service::test_utils::test_cluster;
use walrus_sui::test_utils::LocalOrExternalTestCluster;

// Running this in debug mode can fail with:
// ```
// thread 'localnode::test_cluster_builder' has overflowed its stack
// fatal runtime error: stack overflow
// ```
#[tokio::test]
async fn test_cluster_builder() -> Result<()> {
    let (_sui_cluster_handle, _cluster, _client, _) =
        test_cluster::E2eTestSetupBuilder::new().build().await?;
    match _sui_cluster_handle.as_ref().lock().await.cluster() {
        LocalOrExternalTestCluster::Local { cluster } => {
            println!("{:?}", cluster.get_addresses());
        }
        LocalOrExternalTestCluster::External { rpc_url } => {
            println!("{rpc_url}");
        }
    };
    Ok(())
}

use anyhow::Result;

use walrus_service::test_utils::test_cluster;
use walrus_sui::test_utils::LocalOrExternalTestCluster;

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

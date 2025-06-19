// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{path::PathBuf, sync::Arc};

use sui_move_build::BuildConfig;
use sui_sdk::{
    rpc_types::{ObjectChange, SuiTransactionBlockEffectsAPI, SuiTransactionBlockResponseOptions},
    SuiClient,
    SuiClientBuilder,
};
use sui_types::{
    base_types::{ObjectID, SuiAddress},
    programmable_transaction_builder::ProgrammableTransactionBuilder,
    quorum_driver_types::ExecuteTransactionRequestType,
    transaction::TransactionData,
};
use tokio::sync::Mutex as TokioMutex;
use walrus_sdk::client::Client as WalrusSDKClient;
use walrus_service::test_utils::{test_cluster, StorageNodeHandle, TestCluster};
use walrus_sui::{
    client::SuiContractClient,
    config::load_wallet_context_from_path,
    test_utils::{system_setup::SystemContext, LocalOrExternalTestCluster, TestClusterHandle},
};
use walrus_test_utils::WithTempDir;

#[allow(dead_code)]
pub struct WalrusSitesClusterState {
    pub sui_cluster_handle: Arc<TokioMutex<TestClusterHandle>>,
    pub walrus_cluster: TestCluster<StorageNodeHandle>,
    pub admin_wallet_with_client: WithTempDir<WalrusSDKClient<SuiContractClient>>,
    pub system_context: SystemContext,
    pub walrus_sites_publisher: WalrusSitesPublisher,
    pub walrus_sites_package_id: ObjectID,
    pub sui_execute_client: SuiClient,
}

#[allow(dead_code)]
pub enum WalrusSitesPublisher {
    // We are using:
    // ```
    // load_wallet_context_from_path(
    //     Some(
    //         sui_cluster_handle
    //             .lock()
    //             .await
    //             .wallet_path()
    //             .await
    //             .as_path(),
    //     ),
    //     None,
    // )?
    // ```
    // to sign with this address
    FromSuiClusterHandle(SuiAddress),
}

#[allow(dead_code)]
impl WalrusSitesClusterState {
    // It is a little messy but it gets the job done for now
    pub async fn new() -> anyhow::Result<Self> {
        const PUBLISH_GAS_BUDGET: u64 = 5_000_000_000;
        let (sui_cluster_handle, walrus_cluster, admin_wallet_with_client, system_context) =
            test_cluster::E2eTestSetupBuilder::new().build().await?;
        let rpc_url = sui_cluster_handle.lock().await.rpc_url();
        // println!("rpc_url: {}", rpc_url);
        let sui_execute_client = SuiClientBuilder::default().build(rpc_url).await?;

        // Get RetriableSuiClient from client
        // Note to self 1: Is there a better way to do this?
        // Note to self 2: Should we instead avoid depending on walrus interfaces, and use a regular
        // SuiClient even if it is not "Retriable"?
        let retriable_sui_client = admin_wallet_with_client.inner.sui_client().sui_client();

        // Build package
        let path_buf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("move")
            .join("walrus_site");
        let path = path_buf.as_path();
        let move_build_config = BuildConfig::new_for_testing();
        let compiled_modules = move_build_config.build(path)?;
        let modules_bytes = compiled_modules.get_package_bytes(false);

        // TODO: use a new address for walrus-sites publisher, instead of the first address in the
        // test-cluster
        // Sender and gas info
        let publisher = {
            let lock = sui_cluster_handle.as_ref().lock().await;
            let addresses = get_addresses_from_local(&lock).await;
            *addresses.first().expect("Expected at least 1 address")
        };
        let gas_data = retriable_sui_client
            .select_coins(publisher, None, PUBLISH_GAS_BUDGET as u128, vec![])
            .await?;
        let gas_price = retriable_sui_client.get_reference_gas_price().await?;

        // Tx building
        let mut builder = ProgrammableTransactionBuilder::new();
        let upgrade_cap = builder.publish_upgradeable(
            modules_bytes,
            vec![
                ObjectID::from_hex_literal("0x1").unwrap(),
                ObjectID::from_hex_literal("0x2").unwrap(),
            ],
        );
        builder.transfer_arg(publisher, upgrade_cap);

        let pt = builder.finish();
        let tx_data = TransactionData::new_programmable(
            publisher,
            gas_data.into_iter().map(|c| c.object_ref()).collect(),
            pt,
            PUBLISH_GAS_BUDGET,
            gas_price,
        );

        let signed_tx = load_wallet_context_from_path(
            Some(
                sui_cluster_handle
                    .lock()
                    .await
                    .wallet_path()
                    .await
                    .as_path(),
            ),
            None,
        )?
        .sign_transaction(&tx_data);
        let resp = sui_execute_client
            .quorum_driver_api()
            .execute_transaction_block(
                signed_tx,
                SuiTransactionBlockResponseOptions::default()
                    .with_object_changes()
                    .with_effects(),
                Some(ExecuteTransactionRequestType::WaitForLocalExecution),
            )
            .await?;

        // TODO: I do not like that I am mixing results/errors with aborts
        assert!(resp
            .effects
            .expect("sui_execute_client execute_transaction_block should pass with_effects()")
            .status()
            .is_ok());

        let walrus_sites_package_id = resp
            .object_changes
            .expect(
                "sui_execute_client execute_transaction_block should pass with_object_changes()",
            )
            .into_iter()
            .find_map(|chng| match chng {
                ObjectChange::Published { package_id, .. } => Some(package_id),
                _ => None,
            })
            .expect("Expected published object change");

        Ok(WalrusSitesClusterState {
            sui_cluster_handle,
            walrus_cluster,
            admin_wallet_with_client,
            system_context,
            walrus_sites_publisher: WalrusSitesPublisher::FromSuiClusterHandle(publisher),
            walrus_sites_package_id,
            sui_execute_client,
        })
    }
}

// TODO: use a new address for walrus-sites publisher, instead of the first address in the
// test-cluster.
async fn get_addresses_from_local(cluster: &TestClusterHandle) -> Vec<SuiAddress> {
    let LocalOrExternalTestCluster::Local { cluster } = cluster.cluster() else {
        panic!("Expected Local cluster")
    };
    cluster.get_addresses()
}

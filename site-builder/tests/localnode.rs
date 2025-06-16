// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;

use anyhow::Result;
use sui_move_build::BuildConfig;
use sui_sdk::{
    rpc_types::{SuiTransactionBlockEffectsAPI, SuiTransactionBlockResponseOptions},
    SuiClientBuilder,
};
use sui_types::{
    base_types::{ObjectID, SuiAddress},
    programmable_transaction_builder::ProgrammableTransactionBuilder,
    quorum_driver_types::ExecuteTransactionRequestType,
    transaction::TransactionData,
};
use walrus_service::test_utils::test_cluster;
use walrus_sui::{
    config::load_wallet_context_from_path,
    test_utils::{LocalOrExternalTestCluster, TestClusterHandle},
};

// Running this in `opt-level = 0` mode can fail with:
// ```
// thread 'localnode::test_cluster_builder' has overflowed its stack
// fatal runtime error: stack overflow
// ```
#[tokio::test]
async fn test_print_addresses() -> Result<()> {
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

async fn get_addresses_from_local(cluster: &TestClusterHandle) -> Vec<SuiAddress> {
    let LocalOrExternalTestCluster::Local { cluster } = cluster.cluster() else {
        panic!("Expected Local cluster")
    };
    cluster.get_addresses()
}

#[tokio::test]
async fn publish_walrus_sites() -> Result<()> {
    const PUBLISH_GAS_BUDGET: u64 = 5_000_000_000;
    let (sui_cluster_handle, _cluster, client, _) =
        test_cluster::E2eTestSetupBuilder::new().build().await?;
    // println!("sui_cluster_handle: {:#?}", sui_cluster_handle);
    // println!("_cluster: {:#?}", _cluster);
    // println!("client: {:#?}", client);
    let rpc_url = sui_cluster_handle.lock().await.rpc_url();
    let sui_execute_client = SuiClientBuilder::default().build(rpc_url).await?;

    // Get RetriableSuiClient from client
    let sui_client = client.inner.sui_client().sui_client();

    //  let sui_execute_client = SuiClientBuilder::default()
    //   .build(sui_client)
    //   .await?;
    // let sui_execute_client = SuiClient::new(

    // Build package
    let path_buf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("move")
        .join("walrus_site");
    let path = path_buf.as_path();
    println!("{}", path.to_str().unwrap());
    // let path_buf = ;
    let move_build_config = BuildConfig::new_for_testing();
    let compiled_modules = move_build_config.build(path)?;
    let modules_bytes = compiled_modules.get_package_bytes(false);

    // Sender and gas
    let publisher = {
        let lock = sui_cluster_handle.as_ref().lock().await;
        let addresses = get_addresses_from_local(&lock).await;
        *addresses
            .first()
            .expect("Expected at least 1 address")
    };

    let gas_data = sui_client
        .select_coins(publisher, None, PUBLISH_GAS_BUDGET as u128, vec![])
        .await?;
    let gas_price = sui_client.get_reference_gas_price().await?;

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
            SuiTransactionBlockResponseOptions::default().with_effects(),
            Some(ExecuteTransactionRequestType::WaitForLocalExecution),
        )
        .await?;

    println!("{:#?}", resp.effects.unwrap().status());

    // let res = sui_client.dere
    //
    // let res = client
    //     .quorum_driver_api()
    //     .execute_transaction_block(
    //         Transaction::from_data(tx_data, vec![sig]),
    //         SuiTransactionBlockResponseOptions::new()
    //             .with_effects()
    //             .with_object_changes()
    //             .with_input(),
    //         Some(ExecuteTransactionRequestType::WaitForLocalExecution),
    //     )
    //     .await?;

    Ok(())
}

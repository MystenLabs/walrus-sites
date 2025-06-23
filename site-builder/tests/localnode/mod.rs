// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{fs::File, path::PathBuf, sync::Arc};

use anyhow::{anyhow, bail};
use site_builder::{args::GeneralArgs, config::Config as SitesConfig};
use sui_move_build::BuildConfig;
use sui_sdk::{
    rpc_types::{
        ObjectChange,
        SuiExecutionStatus,
        SuiTransactionBlockEffectsAPI,
        SuiTransactionBlockResponseOptions,
    },
    SuiClient,
    SuiClientBuilder,
};
use sui_types::{
    base_types::ObjectID,
    programmable_transaction_builder::ProgrammableTransactionBuilder,
    quorum_driver_types::ExecuteTransactionRequestType,
    transaction::TransactionData,
};
use tempfile::TempDir;
use tokio::sync::Mutex as TokioMutex;
use walrus_sdk::client::Client as WalrusSDKClient;
use walrus_service::test_utils::{test_cluster, StorageNodeHandle, TestCluster};
use walrus_sui::{
    client::{contract_config::ContractConfig, SuiContractClient},
    test_utils::{new_wallet_on_sui_test_cluster, system_setup::SystemContext, TestClusterHandle},
    wallet::Wallet,
};
use walrus_test_utils::WithTempDir;

#[allow(dead_code)]
pub struct WalrusSitesClusterState {
    // TODO: Specify if this is indeed the walrus publisher
    pub admin_wallet_with_client: WithTempDir<WalrusSDKClient<SuiContractClient>>,
    pub sui_cluster_handle: Arc<TokioMutex<TestClusterHandle>>,
    pub system_context: SystemContext,
    pub walrus_cluster: TestCluster<StorageNodeHandle>,
    pub walrus_sites_publisher: WithTempDir<Wallet>,
}

#[allow(dead_code)]
pub struct TestSetup {
    pub cluster_state: WalrusSitesClusterState,
    pub client: SuiClient,
    pub sites_config: WithTempDir<(SitesConfig, PathBuf)>,
    pub wallet: WithTempDir<Wallet>,
    pub walrus_config: WithTempDir<(ContractConfig, PathBuf)>,
    pub walrus_sites_package_id: ObjectID,
}

#[allow(dead_code)]
impl TestSetup {
    // It is a little messy but it gets the job done for now
    pub async fn new() -> anyhow::Result<Self> {
        let (sui_cluster_handle, walrus_cluster, walrus_sui_admin, system_context) =
            test_cluster::E2eTestSetupBuilder::new().build().await?;
        let rpc_url = sui_cluster_handle.as_ref().lock().await.rpc_url();
        let sui_client = SuiClientBuilder::default().build(rpc_url.clone()).await?;

        // ================================= Publish Walrus-Sites ==================================
        let mut walrus_sites_publisher =
            new_wallet_on_sui_test_cluster(sui_cluster_handle.clone()).await?;
        let walrus_sites_package_id =
            publish_walrus_sites(&sui_client, &mut walrus_sites_publisher.inner).await?;

        // ================================= Create walrus config ==================================
        let walrus_sui_client = walrus_sui_admin.inner.sui_client();
        let walrus_config = create_walrus_config(walrus_sui_client)?;

        // ========================== Create new wallet and sites config ===========================
        let test_wallet =
            new_wallet_with_sui_and_wal(sui_cluster_handle.clone(), walrus_sui_client).await?;

        // Create sites_config
        let sites_config = create_sites_config(
            rpc_url,
            test_wallet.inner.get_config_path().to_path_buf(),
            walrus_sites_package_id,
            walrus_config.inner.1.clone(),
            walrus_sui_client.read_client().get_staking_object_id(),
        )?;

        Ok(TestSetup {
            cluster_state: WalrusSitesClusterState {
                admin_wallet_with_client: walrus_sui_admin,
                sui_cluster_handle,
                system_context,
                walrus_cluster,
                walrus_sites_publisher,
            },
            client: sui_client,
            sites_config,
            wallet: test_wallet,
            walrus_config,
            walrus_sites_package_id,
        })

        // Ok(WalrusSitesClusterState {
        //     sui_cluster_handle: sui_cluster_handle.clone(),
        //     walrus_cluster,
        //     admin_wallet_with_client: walrus_sui_client,
        //     system_context,
        //     walrus_config,
        //     walrus_sites_publisher: WalrusSitesPublisher::FromSuiClusterHandle(
        //         load_wallet_context_from_path(Some(path.as_path()), None)?.active_address()?,
        //     ),
        //     walrus_sites_package_id,
        //     sui_execute_client: sui_client,
        //     sites_config,
        // })
    }
}

async fn publish_walrus_sites(
    sui_client: &SuiClient,
    publisher: &mut Wallet,
) -> anyhow::Result<ObjectID> {
    const PUBLISH_GAS_BUDGET: u64 = 5_000_000_000;

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

    let wallet_active_address = publisher.active_address()?;
    let gas_data = sui_client
        .coin_read_api()
        .select_coins(
            wallet_active_address,
            None,
            PUBLISH_GAS_BUDGET as u128,
            vec![],
        )
        .await?;
    let gas_price = sui_client.read_api().get_reference_gas_price().await?;

    // Tx building
    let mut builder = ProgrammableTransactionBuilder::new();
    let upgrade_cap = builder.publish_upgradeable(
        modules_bytes,
        vec![
            ObjectID::from_hex_literal("0x1").unwrap(),
            ObjectID::from_hex_literal("0x2").unwrap(),
        ],
    );
    builder.transfer_arg(wallet_active_address, upgrade_cap);

    let pt = builder.finish();
    let tx_data = TransactionData::new_programmable(
        wallet_active_address,
        gas_data.into_iter().map(|c| c.object_ref()).collect(),
        pt,
        PUBLISH_GAS_BUDGET,
        gas_price,
    );

    let signed_tx = publisher.sign_transaction(&tx_data);
    let resp = sui_client
        .quorum_driver_api()
        .execute_transaction_block(
            signed_tx,
            SuiTransactionBlockResponseOptions::default()
                .with_object_changes()
                .with_effects(),
            Some(ExecuteTransactionRequestType::WaitForLocalExecution),
        )
        .await?;

    if let SuiExecutionStatus::Failure { error } = resp
        .effects
        .ok_or(anyhow!(
            "No effects in response. Should publish with show_effects: true"
        ))?
        .status()
    {
        bail!("Publishing walrus sites failed with error:\n{error}");
    };

    resp.object_changes
        .ok_or(anyhow!(
            "No object_changes in response. Should publish with show_object_changes: true"
        ))?
        .into_iter()
        .find_map(|chng| match chng {
            ObjectChange::Published { package_id, .. } => Some(package_id),
            _ => None,
        })
        .ok_or(anyhow!("No published package in response."))
}

pub async fn new_wallet_with_sui_and_wal(
    sui_cluster_handle: Arc<TokioMutex<TestClusterHandle>>,
    walrus_sui_client: &SuiContractClient,
) -> anyhow::Result<WithTempDir<Wallet>> {
    const WAL_FUND: u64 = 1000_000_000_000;

    let mut test_wallet = new_wallet_on_sui_test_cluster(sui_cluster_handle.clone()).await?;
    walrus_sui_client
        .send_wal(WAL_FUND, test_wallet.inner.active_address()?)
        .await?;
    Ok(test_wallet)
}

pub fn create_walrus_config(
    walrus_sui_client: &SuiContractClient,
) -> anyhow::Result<WithTempDir<(ContractConfig, PathBuf)>> {
    let read_client = walrus_sui_client.read_client();
    // TODO: create Config structs instead of files.
    let walrus_config = read_client.contract_config();
    let temp_dir = TempDir::new().expect("able to create a temporary directory");
    let walrus_config_path = temp_dir
        .path()
        .to_path_buf()
        .join("walrus_client_config.yaml");
    // tokio::time::sleep(std::time::Duration::from_secs(300)).await;
    let mut walrus_yaml_file = File::create(walrus_config_path.as_path())?;
    serde_yaml::to_writer(&mut walrus_yaml_file, &walrus_config)?;
    Ok(WithTempDir {
        inner: (walrus_config, walrus_config_path.clone()),
        temp_dir,
    })
}

pub fn create_sites_config(
    rpc_url: String,
    wallet_path: PathBuf,
    walrus_sites_package_id: ObjectID,
    walrus_config_path: PathBuf,
    staking_object: ObjectID,
) -> anyhow::Result<WithTempDir<(SitesConfig, PathBuf)>> {
    let temp_dir = TempDir::new().expect("able to create a temporary directory");
    let sites_config_path = temp_dir.path().to_path_buf().join("sites-config.yaml");

    let sites_config = SitesConfig {
        portal: "".to_string(),
        package: walrus_sites_package_id,
        general: GeneralArgs {
            // Do I need this?
            rpc_url: Some(rpc_url),
            wallet: Some(wallet_path),
            walrus_config: Some(walrus_config_path),
            ..Default::default()
        },
        // TODO: DO I need this?
        staking_object: Some(staking_object),
    };
    let mut file = File::create(sites_config_path.as_path())?;
    serde_yaml::to_writer(&mut file, &sites_config)?;
    // TODO: This should probably be done in localnode.
    // Config created:
    // ```
    // portal: ''
    // package: 0x1399dde83b06a80b2eb65f4c529596141bb0723411ce8386d8b2fea1c4cf6f28
    // general:
    //   rpc_url: http://127.0.0.1:62139
    //   wallet: <tmp-dir>/wallet_config.yaml
    //   wallet_env: null
    //   wallet_address: null
    //   walrus_context: null
    //   walrus_binary: walrus
    //   walrus_config: <tmp-dir>/walrus_client_config.yaml
    //   walrus_package: null
    //   gas_budget: 500000000
    // staking_object: 0x992a12ab8fe6d1530bed5832c2875064a40d404c53a00357cc61ffd2cbbe8382
    // ```
    Ok(WithTempDir {
        inner: (sites_config, sites_config_path),
        temp_dir,
    })
}

// TODO: use a new address for walrus-sites publisher, instead of the first address in the
// test-cluster.
// async fn get_addresses_from_local(cluster: &TestClusterHandle) -> Vec<SuiAddress> {
//     let walrus_sui::test_utils::LocalOrExternalTestCluster::Local { cluster } = cluster.cluster() else {
//         panic!("Expected Local cluster")
//     };
//     cluster.get_addresses()
// }

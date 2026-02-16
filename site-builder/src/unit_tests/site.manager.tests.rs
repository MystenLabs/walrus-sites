// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for SiteManager's object cache behavior with stale fullnodes.

use std::path::PathBuf;

use rand::rngs::OsRng;
use sui_config::{node::RunWithRange, Config as _};
use sui_sdk::{
    sui_client_config::{SuiClientConfig, SuiEnv},
    SuiClientBuilder,
};
use sui_types::{
    base_types::ObjectID,
    programmable_transaction_builder::ProgrammableTransactionBuilder,
};
use test_cluster::TestClusterBuilder;
use walrus_sdk::core_utils::backoff::ExponentialBackoffConfig;

use super::SiteManager;
use crate::{args::GeneralArgs, config::Config, retry_client::new_retriable_sui_client};

/// Creates a test Config from a TestCluster's wallet.
fn create_test_config(wallet_path: PathBuf, package_id: ObjectID) -> Config {
    Config {
        portal: "".to_string(),
        package: package_id,
        general: GeneralArgs {
            wallet: Some(wallet_path),
            gas_budget: Some(100_000_000),
            ..Default::default()
        },
        staking_object: None,
    }
}

/// Test that SiteManager's cache is updated after executing a transaction.
#[tokio::test]
async fn test_site_manager_cache_updated_after_transaction() {
    let cluster = TestClusterBuilder::new().build().await;

    // Get wallet config path from the cluster
    let wallet_path = cluster.wallet.config.path().to_path_buf();

    // Create a fake package ID (we won't actually call walrus sites contract)
    let fake_package_id = ObjectID::random();
    let config = create_test_config(wallet_path, fake_package_id);

    // Create SiteManager
    let mut manager = SiteManager::new(config, None, None, None, None)
        .await
        .unwrap();
    assert!(manager.object_cache.is_empty());

    // Get a gas coin and execute a simple transaction
    let address = manager.active_address().unwrap();

    let coins = cluster
        .sui_client()
        .coin_read_api()
        .get_coins(address, None, None, None)
        .await
        .unwrap();
    let coin = coins.data.first().unwrap();
    let gas_ref = coin.object_ref();

    // Build empty PTB
    let ptb = ProgrammableTransactionBuilder::new().finish();

    // Execute via manager's sign_and_send_ptb (which updates cache)
    let _response = manager.sign_and_send_ptb(ptb, gas_ref).await.unwrap();

    // Cache should now contain the gas object with updated version
    assert!(!manager.object_cache.is_empty());
    assert!(manager.object_cache.contains_key(&gas_ref.0));

    let new_coin_ref = cluster
        .sui_client()
        .coin_read_api()
        .get_coins(address, None, None, None)
        .await
        .unwrap()
        .data
        .into_iter()
        .find(|c| c.coin_object_id == gas_ref.0)
        .unwrap()
        .object_ref();

    let cached = *manager.object_cache.get(&gas_ref.0).unwrap();
    assert_eq!(
        cached, new_coin_ref,
        "Expected cached object reference to match newest object reference"
    );
    assert!(
        cached.1 > gas_ref.1,
        "Cached version {:?} should be > initial {:?}",
        cached.1,
        gas_ref.1
    );
}

/// Test that SiteManager's cache protects against stale fullnode data.
///
/// This test verifies:
/// 1. SiteManager executes a tx, updating its cache with the new object version
/// 2. A stale fullnode returns the old object version
/// 3. SiteManager's `verify_object_ref_choose_latest` returns the cached (newer) version
/// 4. SiteManager can execute another tx using stale FN for data but main FN for execution
#[tokio::test]
async fn test_site_manager_cache_protects_against_stale_fullnode() {
    let mut cluster = TestClusterBuilder::new().build().await;

    // Get wallet config path from the cluster
    let wallet_path = cluster.wallet.config.path().to_path_buf();
    let fake_package_id = ObjectID::random();
    let config = create_test_config(wallet_path.clone(), fake_package_id);

    // Create SiteManager with main fullnode
    let mut manager = SiteManager::new(config, None, None, None, None)
        .await
        .unwrap();
    let address = manager.active_address().unwrap();

    // Get initial gas coin version (V0)
    let coins = cluster
        .sui_client()
        .coin_read_api()
        .get_coins(address, None, None, None)
        .await
        .unwrap();
    let coin = coins.data.first().unwrap();
    let initial_gas_ref = coin.object_ref(); // V0

    // Get current checkpoint to stop stale fullnode at
    let current_checkpoint = cluster.fullnode_handle.sui_node.with(|node| {
        node.state()
            .get_checkpoint_store()
            .get_highest_executed_checkpoint_seq_number()
            .unwrap()
            .unwrap_or(0)
    });

    // Spawn stale fullnode that stops at current checkpoint (knows V0, won't see V1)
    let stale_fullnode = cluster
        .start_fullnode_from_config(
            cluster
                .fullnode_config_builder()
                .with_run_with_range(Some(RunWithRange::Checkpoint(current_checkpoint)))
                .build(&mut OsRng, cluster.swarm.config()),
        )
        .await;

    // Step 2: SiteManager executes a simple tx with the gas-object using main fullnode
    let ptb = ProgrammableTransactionBuilder::new().finish();

    let _response = manager
        .sign_and_send_ptb(ptb, initial_gas_ref)
        .await
        .unwrap();

    // Cache should now contain the gas object with updated version (V1)
    assert!(manager.object_cache.contains_key(&initial_gas_ref.0));
    let cached_gas_ref = *manager.object_cache.get(&initial_gas_ref.0).unwrap();
    assert!(
        cached_gas_ref.1 > initial_gas_ref.1,
        "Cached version {:?} should be > initial {:?}",
        cached_gas_ref.1,
        initial_gas_ref.1
    );

    // Step 3 & 4: Get coin from stale fullnode - should return old version (V0)
    let stale_client = SuiClientBuilder::default()
        .build(&stale_fullnode.rpc_url)
        .await
        .unwrap();

    let stale_coins = stale_client
        .coin_read_api()
        .get_coins(address, None, None, None)
        .await
        .unwrap();
    let stale_coin = stale_coins
        .data
        .iter()
        .find(|c| c.coin_object_id == initial_gas_ref.0)
        .unwrap();
    let stale_gas_ref = stale_coin.object_ref();

    // Step 5: Verify stale fullnode returns old version, cache has new version
    assert_eq!(
        stale_gas_ref.1, initial_gas_ref.1,
        "Stale fullnode should return initial version V0"
    );
    assert!(
        cached_gas_ref.1 > stale_gas_ref.1,
        "Cached version {:?} should be > stale version {:?}",
        cached_gas_ref.1,
        stale_gas_ref.1
    );

    // Verify that verify_object_ref_choose_latest returns cached (newer) version
    let chosen_ref = manager
        .verify_object_ref_choose_latest(stale_gas_ref)
        .unwrap();
    assert_eq!(
        chosen_ref, cached_gas_ref,
        "verify_object_ref_choose_latest should return cached version, not stale"
    );

    // Step 6: SiteManager executes a new tx using:
    // - Stale fullnode wallet for data queries (would return V0)
    // - But cache overrides to V1
    // - Main fullnode client for execution

    // Create a new wallet config pointing to stale fullnode
    let stale_wallet_path = wallet_path.parent().unwrap().join("stale_wallet.yaml");
    let mut stale_wallet_config: SuiClientConfig = SuiClientConfig::load(&wallet_path).unwrap();
    stale_wallet_config.envs = vec![SuiEnv {
        alias: "stale".to_string(),
        rpc: stale_fullnode.rpc_url.clone(),
        ws: None,
        basic_auth: None,
        chain_id: None,
    }];
    stale_wallet_config.active_env = Some("stale".to_string());
    stale_wallet_config
        .persisted(&stale_wallet_path)
        .save()
        .unwrap();

    // Create new SiteManager with stale wallet config, but copy the cache
    let stale_config = create_test_config(stale_wallet_path, fake_package_id);
    let mut stale_manager = SiteManager::new(stale_config, None, None, None, None)
        .await
        .unwrap();

    // Copy the cache from original manager (simulating persistent cache)
    stale_manager.object_cache = manager.object_cache.clone();

    // Create a RetriableSuiClient pointing to main (non-stale) fullnode for execution
    // using the original manager's wallet which still points to main fullnode
    let main_retry_client = new_retriable_sui_client(
        &manager.wallet.config.get_env(&None).unwrap().rpc,
        ExponentialBackoffConfig::default(),
    )
    .unwrap();

    // stale_manager.wallet queries stale FN and returns V0
    let stale_wallet_gas_ref = stale_manager
        .wallet
        .get_object_ref(initial_gas_ref.0)
        .await
        .unwrap();
    assert_eq!(
        stale_wallet_gas_ref.1, initial_gas_ref.1,
        "Stale wallet should return V0"
    );

    // But verify_object_ref_choose_latest returns V1 from cache
    let gas_ref_for_tx = stale_manager
        .verify_object_ref_choose_latest(stale_wallet_gas_ref)
        .unwrap();
    assert_eq!(
        gas_ref_for_tx, cached_gas_ref,
        "verify_object_ref_choose_latest should return cached V1, not stale V0"
    );

    // Execute another tx using the cache-corrected gas ref (V1) through main fullnode
    let ptb2 = ProgrammableTransactionBuilder::new().finish();
    let _response2 = crate::util::sign_and_send_ptb(
        stale_manager.active_address().unwrap(),
        &stale_manager.wallet,
        &main_retry_client,
        ptb2,
        gas_ref_for_tx,
        stale_manager.config.gas_budget(),
        &mut stale_manager.object_cache,
    )
    .await
    .unwrap();

    // Verify the transaction succeeded and cache was updated to V2
    let final_cached_ref = *stale_manager.object_cache.get(&initial_gas_ref.0).unwrap();
    assert!(
        final_cached_ref.1 > cached_gas_ref.1,
        "Final cached version {:?} should be > previous cached version {:?}",
        final_cached_ref.1,
        cached_gas_ref.1
    );
}

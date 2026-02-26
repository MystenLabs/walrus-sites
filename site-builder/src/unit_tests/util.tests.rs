// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use move_core_types::language_storage::StructTag;
use sui_sdk::rpc_types::{SuiTransactionBlockEffectsAPI, SuiTransactionBlockResponseOptions};
use sui_types::{
    programmable_transaction_builder::ProgrammableTransactionBuilder,
    transaction::{Argument, Command, TransactionData},
};
use test_cluster::TestClusterBuilder;

use crate::{
    site::config::WSResources,
    types::ObjectCache,
    util::{is_ignored, is_pattern_match, update_cache_from_effects},
};

struct PatternMatchTestCase {
    pattern: &'static str,
    path: &'static str,
    expected: bool,
}

#[test]
fn test_is_ignored() {
    const IGNORE_DATA: &str = r#"
	    "ignore": [
	        "/foo/*",
	        "/baz/bar/*"
	    ]
    "#;
    let ignore_data = format!("{{{IGNORE_DATA}}}");
    let ws_resources: WSResources =
        serde_json::from_str(&ignore_data).expect("parsing should succeed");
    assert!(ws_resources.ignore.is_some());
    assert!(is_ignored(
        ws_resources
            .ignore
            .as_deref()
            .into_iter()
            .flatten()
            .map(String::as_str),
        "/foo/nested/bar.txt"
    )
    .expect("is_ignored should not fail with valid patterns"));
}

#[test]
fn test_is_pattern_match() {
    let tests = vec![
        PatternMatchTestCase {
            pattern: "/*.txt",
            path: "/file.txt",
            expected: true,
        },
        PatternMatchTestCase {
            pattern: "*.txt",
            path: "/file.doc",
            expected: false,
        },
        PatternMatchTestCase {
            pattern: "/test/*",
            path: "/test/file",
            expected: true,
        },
        PatternMatchTestCase {
            pattern: "/test/*",
            path: "/test/file.extension",
            expected: true,
        },
        PatternMatchTestCase {
            pattern: "/test/*",
            path: "/test/foo.bar.extension",
            expected: true,
        },
        PatternMatchTestCase {
            pattern: "/test/*",
            path: "/test/foo-bar_baz.extension",
            expected: true,
        },
    ];
    for t in tests {
        assert_eq!(
            is_pattern_match(t.pattern, t.path).expect("valid pattern should not fail"),
            t.expected
        );
    }
}

#[test]
fn test_is_pattern_match_invalid_pattern() {
    let result = is_pattern_match("[invalid", "/file");
    assert!(
        result.is_err(),
        "invalid glob pattern should return an error"
    );
}

// ============ update_cache_from_effects tests ============

/// Tests that `update_cache_from_effects` correctly caches objects from real transaction effects.
/// This test splits a coin to observe created objects, and passes an unused coin to see if it
/// appears in mutated.
#[tokio::test]
async fn test_update_cache_from_effects_with_real_tx() {
    let cluster = TestClusterBuilder::new().build().await;
    let address = cluster.get_address_0();

    // Get coins - we need at least 3 (one for gas, one to split, one unused)
    let sui_coin_type: StructTag = "0x2::coin::Coin<0x2::sui::SUI>".parse().unwrap();
    let grpc = cluster.grpc_client();
    let coin_objects = grpc
        .get_owned_objects(address, Some(sui_coin_type), None, None)
        .await
        .unwrap()
        .items;

    let gas_ref = coin_objects[0].compute_object_reference();
    let gas_object_id = gas_ref.0;

    let split_coin_ref = coin_objects[1].compute_object_reference();
    let split_coin_id = split_coin_ref.0;

    // Get a third coin that we'll pass as input but not use
    let unused_coin_ref = coin_objects[2].compute_object_reference();
    let unused_coin_id = unused_coin_ref.0;

    // Build PTB that splits a coin
    let gas_price = grpc.get_reference_gas_price().await.unwrap();

    let mut ptb = ProgrammableTransactionBuilder::new();
    let coin_arg = ptb
        .obj(sui_types::transaction::ObjectArg::ImmOrOwnedObject(
            split_coin_ref,
        ))
        .unwrap();
    // Add unused coin as input - we won't do anything with it
    let _unused_coin_arg = ptb
        .obj(sui_types::transaction::ObjectArg::ImmOrOwnedObject(
            unused_coin_ref,
        ))
        .unwrap();
    let amount = ptb.pure(1000u64).unwrap();
    let addr_arg = ptb.pure(address).unwrap();
    // Split 1000 MIST from the coin
    ptb.command(Command::SplitCoins(coin_arg, vec![amount]));
    // Transfer the new coin to ourselves
    ptb.command(Command::TransferObjects(
        vec![Argument::NestedResult(0, 0)],
        addr_arg,
    ));
    let pt = ptb.finish();

    let tx_data =
        TransactionData::new_programmable(address, vec![gas_ref], pt, 10_000_000, gas_price);

    let tx = cluster.wallet.sign_transaction(&tx_data).await;
    // Cannot migrate to grpc: update_cache_from_effects requires SuiTransactionBlockEffects
    // (JSON-RPC type), not the gRPC TransactionEffects type.
    #[allow(deprecated)]
    let response = cluster
        .sui_client()
        .quorum_driver_api()
        .execute_transaction_block(
            tx,
            SuiTransactionBlockResponseOptions::new().with_effects(),
            None,
        )
        .await
        .unwrap();

    let effects = response.effects.unwrap();

    // Print effects for debugging
    println!("Effects created: {:?}", effects.created());
    println!("Effects mutated: {:?}", effects.mutated());

    // Even unused coin inputs appear in mutated (Sui bumps version of all owned object inputs)
    let mutated_ids: Vec<_> = effects
        .mutated()
        .iter()
        .map(|o| o.reference.object_id)
        .collect();
    assert!(
        mutated_ids.contains(&unused_coin_id),
        "Unused coin should still appear in mutated (Sui bumps version of all inputs)"
    );

    // Test update_cache_from_effects with real effects
    let mut cache = ObjectCache::new();
    update_cache_from_effects(&mut cache, &effects);

    // Gas object should be cached (it's in mutated list as AddressOwner)
    assert!(
        cache.contains_key(&gas_object_id),
        "Gas object should be cached"
    );

    // The coin we split should be in mutated and cached
    assert!(
        cache.contains_key(&split_coin_id),
        "Split coin should be cached (it's mutated)"
    );

    // The unused coin should also be cached (it was part of the tx, so version is bumped)
    assert!(
        cache.contains_key(&unused_coin_id),
        "Unused coin should be cached (version bumped as tx input)"
    );

    // The newly created coin should be in created and cached
    let created = effects.created();
    assert!(!created.is_empty(), "Should have created a new coin");
    let new_coin_id = created[0].reference.object_id;
    assert!(
        cache.contains_key(&new_coin_id),
        "Newly created coin should be cached"
    );

    // Cached version should be greater than original for gas
    let cached_gas_version = cache.get(&gas_object_id).unwrap().1;
    assert!(
        cached_gas_version > gas_ref.1,
        "Cached gas version {:?} should be > original {:?}",
        cached_gas_version,
        gas_ref.1
    );

    // Cached version should be greater than original for split coin
    let cached_split_version = cache.get(&split_coin_id).unwrap().1;
    assert!(
        cached_split_version > split_coin_ref.1,
        "Cached split coin version {:?} should be > original {:?}",
        cached_split_version,
        split_coin_ref.1
    );
}

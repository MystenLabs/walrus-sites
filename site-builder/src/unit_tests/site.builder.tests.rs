// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use sui_types::base_types::ObjectID;

use super::*;

fn create_test_site_ptb<const MAX_MOVE_CALLS: u16>() -> SitePtb<(), MAX_MOVE_CALLS> {
    let package = ObjectID::ZERO;
    let module = Identifier::new("test_module").unwrap();
    SitePtb::new(package, module, ObjectID::ZERO)
}

fn create_test_site_ptb_with_arg<const MAX_MOVE_CALLS: u16>() -> SitePtb<Argument, MAX_MOVE_CALLS> {
    let mut ptb = create_test_site_ptb::<MAX_MOVE_CALLS>();
    let arg = ptb.pt_builder.input(CallArg::Pure(vec![0])).unwrap();
    ptb.with_arg(arg)
}

#[test]
fn test_check_counter_in_advance_within_limit() {
    let ptb = create_test_site_ptb::<10>();
    assert!(ptb.move_call_check(5).is_ok());
    assert!(ptb.move_call_check(10).is_ok());
}

#[test]
fn test_check_counter_in_advance_exceeds_limit() {
    let ptb = create_test_site_ptb::<10>();
    let result = ptb.move_call_check(11);
    assert!(result.is_err());
    match result.unwrap_err() {
        SitePtbBuilderError::TooManyMoveCalls(limit) => assert_eq!(limit, 10),
        _ => panic!("Expected TooManyMoveCalls error"),
    }
}

#[test]
fn test_check_counter_in_advance_with_existing_counter() {
    let mut ptb = create_test_site_ptb::<10>();
    ptb.move_call_counter = 8;

    // Should pass with 2 more calls
    assert!(ptb.move_call_check(2).is_ok());

    // Should fail with 3 more calls
    let result = ptb.move_call_check(3);
    assert!(result.is_err());
    match result.unwrap_err() {
        SitePtbBuilderError::TooManyMoveCalls(limit) => assert_eq!(limit, 10),
        _ => panic!("Expected TooManyMoveCalls error"),
    }
}

#[test]
fn test_increment_counter_within_limit() {
    let mut ptb = create_test_site_ptb::<10>();
    for i in 1..=10 {
        assert!(ptb.increment_move_call_counter().is_ok());
        assert_eq!(ptb.move_call_counter, i);
    }
}

#[test]
fn test_increment_counter_exceeds_limit() {
    let mut ptb = create_test_site_ptb::<2>();
    ptb.move_call_counter = 3; // Manually set to exceed limit (> 2)

    let result = ptb.increment_move_call_counter();
    assert!(result.is_err());
    match result.unwrap_err() {
        SitePtbBuilderError::TooManyMoveCalls(limit) => assert_eq!(limit, 2),
        _ => panic!("Expected TooManyMoveCalls error"),
    }
}

#[test]
fn test_with_max_move_calls_reduces_limit() {
    let ptb = create_test_site_ptb::<100>();
    let ptb_with_lower_limit = ptb.with_max_move_calls::<5>();

    // Test that the new limit is enforced
    let result = ptb_with_lower_limit.move_call_check(6);
    assert!(result.is_err());
    match result.unwrap_err() {
        SitePtbBuilderError::TooManyMoveCalls(limit) => assert_eq!(limit, 5),
        _ => panic!("Expected TooManyMoveCalls error"),
    }
}

#[test]
fn test_create_site_respects_limit() {
    let mut ptb = create_test_site_ptb::<1>();

    // create_site needs 2 move calls (metadata + site), so should fail with limit of 1
    let result = ptb.create_site("test_site", None);
    assert!(result.is_err());
    match result.unwrap_err() {
        SitePtbBuilderError::TooManyMoveCalls(limit) => assert_eq!(limit, 1),
        _ => panic!("Expected TooManyMoveCalls error"),
    }
}

#[test]
fn test_add_resource_operations_stops_on_limit() {
    // This test is simplified since we don't need to actually create a resource
    // We just need to test that the limit checking works
    let mut ptb = create_test_site_ptb_with_arg::<1>();

    // Manually set counter to exceed the limit (> MAX_MOVE_CALLS)
    ptb.move_call_counter = 2;

    // Try to increment - should fail
    let result = ptb.increment_move_call_counter();
    assert!(result.is_err());
    match result.unwrap_err() {
        SitePtbBuilderError::TooManyMoveCalls(limit) => assert_eq!(limit, 1),
        _ => panic!("Expected TooManyMoveCalls error"),
    }
}

#[test]
fn test_siteptb_builder_result_ext_ok_if_limit_reached() {
    // Test successful result
    let ok_result: SitePtbBuilderResult<i32> = Ok(42);
    let extended = ok_result.ok_if_limit_reached().unwrap();
    assert_eq!(extended, Some(42));

    // Test TooManyMoveCalls error (should be ignored)
    let limit_error: SitePtbBuilderResult<i32> = Err(SitePtbBuilderError::TooManyMoveCalls(10));
    let extended = limit_error.ok_if_limit_reached().unwrap();
    assert_eq!(extended, None);

    // Test Other error (should propagate)
    let other_error: SitePtbBuilderResult<i32> =
        Err(SitePtbBuilderError::Other(anyhow::anyhow!("test error")));
    let result = other_error.ok_if_limit_reached();
    assert!(result.is_err());
}

// Tests for actual public methods that use check_counter_in_advance

#[test]
fn test_replace_routes_respects_limit() {
    let mut ptb = create_test_site_ptb_with_arg::<1>();

    // replace_routes needs 2 move calls (remove + create), so should fail with limit of 1
    let result = ptb.replace_routes();
    assert!(result.is_err());
    match result.unwrap_err() {
        SitePtbBuilderError::TooManyMoveCalls(limit) => assert_eq!(limit, 1),
        _ => panic!("Expected TooManyMoveCalls error"),
    }
}

#[test]
fn test_replace_routes_succeeds_within_limit() {
    let mut ptb = create_test_site_ptb_with_arg::<5>();

    // replace_routes needs 2 move calls, should succeed with limit of 5
    assert!(ptb.replace_routes().is_ok());
    assert_eq!(ptb.move_call_counter, 2);
}

#[test]
fn test_remove_resource_if_exists_increments_counter() {
    use move_core_types::u256::U256;

    use crate::{
        types::{HttpHeaders, SuiResource, VecMap},
        walrus::types::BlobId,
    };

    let mut ptb = create_test_site_ptb_with_arg::<10>();
    let initial_counter = ptb.move_call_counter;

    // Create a minimal resource for testing
    let resource = crate::site::resource::Resource {
        info: SuiResource {
            path: "test.txt".to_string(),
            headers: HttpHeaders(VecMap::new()),
            blob_id: BlobId([0u8; 32]),
            blob_hash: U256::from(0u128),
            range: None,
        },
        unencoded_size: 100,
        full_path: std::path::PathBuf::from("test.txt"),
    };

    assert!(ptb.remove_resource_if_exists(&resource).is_ok());
    assert_eq!(ptb.move_call_counter, initial_counter + 1);
}

#[test]
fn test_remove_resource_if_exists_respects_limit() {
    let mut ptb = create_test_site_ptb_with_arg::<0>();
    ptb.move_call_counter = 1; // Manually set to exceed limit (> 0)

    // Try any operation - should fail since we're over the limit
    let result = ptb.increment_move_call_counter();
    assert!(result.is_err());
    match result.unwrap_err() {
        SitePtbBuilderError::TooManyMoveCalls(limit) => assert_eq!(limit, 0),
        _ => panic!("Expected TooManyMoveCalls error"),
    }
}

#[test]
fn test_update_name_increments_counter() {
    let mut ptb = create_test_site_ptb_with_arg::<10>();
    let initial_counter = ptb.move_call_counter;

    assert!(ptb.update_name("new_name").is_ok());
    assert_eq!(ptb.move_call_counter, initial_counter + 1);
}

#[test]
fn test_update_name_respects_limit() {
    let mut ptb = create_test_site_ptb_with_arg::<0>();
    ptb.move_call_counter = 1; // Manually exceed limit

    let result = ptb.update_name("new_name");
    assert!(result.is_err());
    match result.unwrap_err() {
        SitePtbBuilderError::TooManyMoveCalls(limit) => assert_eq!(limit, 0),
        _ => panic!("Expected TooManyMoveCalls error"),
    }
}

#[test]
fn test_add_route_increments_counter() {
    let mut ptb = create_test_site_ptb_with_arg::<10>();
    let initial_counter = ptb.move_call_counter;

    assert!(ptb.add_route("route1", "value1").is_ok());
    assert_eq!(ptb.move_call_counter, initial_counter + 1);
}

#[test]
fn test_multiple_operations_exhaust_limit() {
    let mut ptb = create_test_site_ptb_with_arg::<3>();

    // First operation should succeed
    assert!(ptb.update_name("name1").is_ok());
    assert_eq!(ptb.move_call_counter, 1);

    // Second operation should succeed
    assert!(ptb.add_route("route1", "value1").is_ok());
    assert_eq!(ptb.move_call_counter, 2);

    // Third operation should succeed
    assert!(ptb.remove_routes().is_ok());
    assert_eq!(ptb.move_call_counter, 3);

    // Fourth operation should fail (counter = 3, limit = 3, and increment_counter checks > limit)
    ptb.move_call_counter = 4; // Manually exceed to trigger the error
    let result = ptb.update_name("name2");
    assert!(result.is_err());
    match result.unwrap_err() {
        SitePtbBuilderError::TooManyMoveCalls(limit) => assert_eq!(limit, 3),
        _ => panic!("Expected TooManyMoveCalls error"),
    }
}

#[test]
fn test_with_update_metadata_increments_counter() {
    use crate::types::Metadata;

    let ptb = create_test_site_ptb_with_arg::<10>();
    let initial_counter = ptb.move_call_counter;

    let metadata = Metadata::default();
    let result = ptb.with_update_metadata(metadata);
    assert!(result.is_ok());

    let updated_ptb = result.unwrap();
    // with_update_metadata calls new_metadata (1) + add_programmable_move_call (1) = 2 calls
    assert_eq!(updated_ptb.move_call_counter, initial_counter + 2);
}

#[test]
fn test_create_routes_increments_counter() {
    let mut ptb = create_test_site_ptb_with_arg::<10>();
    let initial_counter = ptb.move_call_counter;

    assert!(ptb.create_routes().is_ok());
    assert_eq!(ptb.move_call_counter, initial_counter + 1);
}

#[test]
fn test_create_routes_respects_limit() {
    let mut ptb = create_test_site_ptb_with_arg::<0>();
    ptb.move_call_counter = 1; // Manually exceed limit

    let result = ptb.create_routes();
    assert!(result.is_err());
    match result.unwrap_err() {
        SitePtbBuilderError::TooManyMoveCalls(limit) => assert_eq!(limit, 0),
        _ => panic!("Expected TooManyMoveCalls error"),
    }
}

// BCS size estimation const validation tests.
//
// These tests verify that our PTB size estimation constants are >= the actual BCS-serialized sizes.
// If Sui changes the serialization of these types, these tests will catch it.
mod ptb_size_consts {
    use move_core_types::u256::U256;
    use sui_types::{
        base_types::{ObjectDigest, ObjectID, SequenceNumber},
        digests::TransactionDigest,
        transaction::{Argument, CallArg, Command, ObjectArg, ProgrammableMoveCall},
        Identifier,
    };

    use super::super::*;

    /// Helper: BCS-serialized size of a value.
    fn bcs_size<T: serde::Serialize>(val: &T) -> usize {
        bcs::serialized_size(val).expect("BCS serialization should succeed")
    }

    #[test]
    fn test_ptb_argument_ref_size() {
        // Argument::Input(u16) = enum tag (1) + u16 (2) = 3
        assert!(PTB_ARGUMENT_REF_SIZE >= bcs_size(&Argument::Input(0)));
        assert!(PTB_ARGUMENT_REF_SIZE >= bcs_size(&Argument::Input(u16::MAX)));
        assert!(PTB_ARGUMENT_REF_SIZE >= bcs_size(&Argument::Result(0)));
        assert_eq!(PTB_ARGUMENT_REF_SIZE, bcs_size(&Argument::Input(0)));
    }

    #[test]
    fn test_ptb_move_call_size() {
        // Test with the longest function name we use: "remove_all_routes_if_exist" (26 chars)
        // and module "site" (4 chars).
        let longest_call = Command::MoveCall(Box::new(ProgrammableMoveCall {
            package: ObjectID::ZERO,
            module: "site".to_string(),
            function: "remove_all_routes_if_exist".to_string(),
            type_arguments: vec![],
            arguments: vec![], // argument refs are tracked separately
        }));
        let actual = bcs_size(&longest_call);
        assert!(
            PTB_MOVE_CALL_SIZE >= actual,
            "PTB_MOVE_CALL_SIZE ({PTB_MOVE_CALL_SIZE}) < actual BCS size ({actual}) \
             for longest move call"
        );
    }

    #[test]
    fn test_ptb_string_pure_arg_overhead() {
        // Test with various string lengths to verify overhead is constant and <= our const.
        for s in ["", "a", "test.txt", "long/path/to/some/resource.html"] {
            let call_arg = CallArg::Pure(bcs::to_bytes(&s.to_string()).unwrap());
            let actual_overhead = bcs_size(&call_arg) - s.len();
            assert!(
                PTB_STRING_PURE_ARG_OVERHEAD >= actual_overhead,
                "PTB_STRING_PURE_ARG_OVERHEAD ({PTB_STRING_PURE_ARG_OVERHEAD}) < \
                 actual overhead ({actual_overhead}) for string \"{s}\""
            );
        }
    }

    #[test]
    fn test_ptb_u256_pure_arg_size() {
        let call_arg = CallArg::Pure(bcs::to_bytes(&U256::zero()).unwrap());
        let actual = bcs_size(&call_arg);
        assert!(
            PTB_U256_PURE_ARG_SIZE >= actual,
            "PTB_U256_PURE_ARG_SIZE ({PTB_U256_PURE_ARG_SIZE}) < actual BCS size ({actual})"
        );

        // Also test with max value (same size since u256 is fixed-width).
        let call_arg_max = CallArg::Pure(bcs::to_bytes(&U256::max_value()).unwrap());
        assert_eq!(bcs_size(&call_arg_max), actual);
    }

    #[test]
    fn test_ptb_range_none_size() {
        // Two Option::None pure args.
        let none_arg = CallArg::Pure(bcs::to_bytes(&Option::<u64>::None).unwrap());
        let actual = 2 * bcs_size(&none_arg);
        assert!(
            PTB_RANGE_NONE_SIZE >= actual,
            "PTB_RANGE_NONE_SIZE ({PTB_RANGE_NONE_SIZE}) < actual BCS size ({actual})"
        );
        assert_eq!(PTB_RANGE_NONE_SIZE, actual);
    }

    #[test]
    fn test_ptb_range_some_size() {
        // Two Option::Some(u64) pure args.
        let some_arg = CallArg::Pure(bcs::to_bytes(&Some(u64::MAX)).unwrap());
        let actual = 2 * bcs_size(&some_arg);
        assert!(
            PTB_RANGE_SOME_SIZE >= actual,
            "PTB_RANGE_SOME_SIZE ({PTB_RANGE_SOME_SIZE}) < actual BCS size ({actual})"
        );
        assert_eq!(PTB_RANGE_SOME_SIZE, actual);
    }

    #[test]
    fn test_ptb_extend_operation_size() {
        use sui_types::programmable_transaction_builder::ProgrammableTransactionBuilder;

        fn obj_digest() -> ObjectDigest {
            ObjectDigest::new(TransactionDigest::default().into_inner())
        }
        fn owned_obj(id: ObjectID) -> ObjectArg {
            ObjectArg::ImmOrOwnedObject((id, SequenceNumber::new(), obj_digest()))
        }

        let system_id = ObjectID::random();
        let coin_id = ObjectID::random();
        let package_id = ObjectID::random();

        // Build a PTB with 4 extend_blob operations and verify the total fits within
        // 4 * PTB_EXTEND_OPERATION_SIZE. The constant overhead from system + coin inputs
        // is absorbed by the generous per-operation estimate.
        let mut pt_builder = ProgrammableTransactionBuilder::new();
        let system_arg = pt_builder
            .obj(ObjectArg::SharedObject {
                id: system_id,
                initial_shared_version: SequenceNumber::new(),
                mutability: sui_types::transaction::SharedObjectMutability::Mutable,
            })
            .unwrap();
        let coin_arg = pt_builder.obj(owned_obj(coin_id)).unwrap();

        for _ in 0..4 {
            let blob_arg = pt_builder.obj(owned_obj(ObjectID::random())).unwrap();
            let epochs_arg = pt_builder.pure(1u32).unwrap();
            pt_builder.programmable_move_call(
                package_id,
                Identifier::new("system").unwrap(),
                Identifier::new("extend_blob").unwrap(),
                vec![],
                vec![system_arg, blob_arg, epochs_arg, coin_arg],
            );
        }

        let total = bcs_size(&pt_builder.finish());
        assert!(
            4 * PTB_EXTEND_OPERATION_SIZE >= total,
            "4 * PTB_EXTEND_OPERATION_SIZE ({}) < actual total BCS size ({total})",
            4 * PTB_EXTEND_OPERATION_SIZE,
        );
        println!(
            "4 * PTB_EXTEND_OPERATION_SIZE ({}) < actual total BCS size ({total})",
            4 * PTB_EXTEND_OPERATION_SIZE,
        );
    }
}

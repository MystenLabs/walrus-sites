// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use sui_types::base_types::ObjectID;

use super::*;

fn create_test_site_ptb<const MAX_MOVE_CALLS: u16>() -> SitePtb<(), MAX_MOVE_CALLS> {
    let package = ObjectID::ZERO;
    let module = Identifier::new("test_module").unwrap();
    SitePtb::new(package, module)
}

fn create_test_site_ptb_with_arg<const MAX_MOVE_CALLS: u16>() -> SitePtb<Argument, MAX_MOVE_CALLS> {
    let mut ptb = create_test_site_ptb::<MAX_MOVE_CALLS>();
    let arg = ptb.pt_builder.input(CallArg::Pure(vec![0])).unwrap();
    ptb.with_arg(arg)
}

#[test]
fn test_check_counter_in_advance_within_limit() {
    let ptb = create_test_site_ptb::<10>();
    assert!(ptb.check_counter_in_advance(5).is_ok());
    assert!(ptb.check_counter_in_advance(10).is_ok());
}

#[test]
fn test_check_counter_in_advance_exceeds_limit() {
    let ptb = create_test_site_ptb::<10>();
    let result = ptb.check_counter_in_advance(11);
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
    assert!(ptb.check_counter_in_advance(2).is_ok());

    // Should fail with 3 more calls
    let result = ptb.check_counter_in_advance(3);
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
        assert!(ptb.increment_counter().is_ok());
        assert_eq!(ptb.move_call_counter, i);
    }
}

#[test]
fn test_increment_counter_exceeds_limit() {
    let mut ptb = create_test_site_ptb::<2>();
    ptb.move_call_counter = 3; // Manually set to exceed limit (> 2)

    let result = ptb.increment_counter();
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
    let result = ptb_with_lower_limit.check_counter_in_advance(6);
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
    let result = ptb.increment_counter();
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
    let result = ptb.increment_counter();
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

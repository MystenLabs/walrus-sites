// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

#[test_only]
module walrus_site::redirects_tests;

use walrus_site::redirects::{
    Self,
    EInvalidRedirectStatusCode,
    ELocationsLength,
    EStatusCodesLengths
};

// === filled ===

#[test]
fun test_filled_single() {
    let redirects = redirects::filled(
        vector[b"/old".to_string()],
        vector[b"/new".to_string()],
        vector[301],
    );
    assert!(redirects.length() == 1);
    let (location, status_code) = redirects.get(&b"/old".to_string());
    assert!(*location == b"/new".to_string());
    assert!(status_code == 301);
}

#[test]
fun test_filled_multiple() {
    let redirects = redirects::filled(
        vector[b"/a".to_string(), b"/b".to_string()],
        vector[b"/x".to_string(), b"/y".to_string()],
        vector[301, 308],
    );
    assert!(redirects.length() == 2);
    let (location, status_code) = redirects.get(&b"/a".to_string());
    assert!(*location == b"/x".to_string());
    assert!(status_code == 301);
    let (location, status_code) = redirects.get(&b"/b".to_string());
    assert!(*location == b"/y".to_string());
    assert!(status_code == 308);
}

#[test]
fun test_filled_empty_vectors() {
    let redirects = redirects::filled(vector[], vector[], vector[]);
    assert!(redirects.length() == 0);
}

#[test]
fun test_filled_external_url() {
    let redirects = redirects::filled(
        vector[b"/docs".to_string()],
        vector[b"https://docs.example.com".to_string()],
        vector[302],
    );
    let (location, status_code) = redirects.get(&b"/docs".to_string());
    assert!(*location == b"https://docs.example.com".to_string());
    assert!(status_code == 302);
}

// === fill ===

#[test]
fun test_fill_on_existing() {
    let mut redirects = redirects::empty();
    assert!(redirects.length() == 0);
    redirects.fill(
        vector[b"/old".to_string()],
        vector[b"/new".to_string()],
        vector[308],
    );
    assert!(redirects.length() == 1);
    let (location, status_code) = redirects.get(&b"/old".to_string());
    assert!(*location == b"/new".to_string());
    assert!(status_code == 308);
}

#[test]
#[expected_failure(abort_code = ELocationsLength)]
fun test_fill_mismatched_locations_length() {
    redirects::filled(
        vector[b"/a".to_string(), b"/b".to_string()],
        vector[b"/x".to_string()],
        vector[301, 301],
    );
}

#[test]
#[expected_failure(abort_code = EStatusCodesLengths)]
fun test_fill_mismatched_status_codes_length() {
    redirects::filled(
        vector[b"/a".to_string(), b"/b".to_string()],
        vector[b"/x".to_string(), b"/y".to_string()],
        vector[301],
    );
}

// === insert / remove ===

#[test]
fun test_insert_and_remove() {
    let mut redirects = redirects::empty();
    redirects.insert(b"/old".to_string(), b"/new".to_string(), 301);
    assert!(redirects.length() == 1);

    let (path, location, status_code) = redirects.remove(&b"/old".to_string());
    assert!(path == b"/old".to_string());
    assert!(location == b"/new".to_string());
    assert!(status_code == 301);
    assert!(redirects.length() == 0);
}

#[test]
fun test_insert_multiple() {
    let mut redirects = redirects::empty();
    redirects.insert(b"/a".to_string(), b"/1".to_string(), 301);
    redirects.insert(b"/b".to_string(), b"/2".to_string(), 302);
    redirects.insert(b"/c".to_string(), b"/3".to_string(), 307);
    assert!(redirects.length() == 3);

    let (location, status_code) = redirects.get(&b"/b".to_string());
    assert!(*location == b"/2".to_string());
    assert!(status_code == 302);
}

// === status code validation ===

#[test]
fun test_all_valid_status_codes_via_insert() {
    let mut redirects = redirects::empty();
    redirects.insert(b"/a".to_string(), b"/1".to_string(), 301);
    redirects.insert(b"/b".to_string(), b"/2".to_string(), 302);
    redirects.insert(b"/c".to_string(), b"/3".to_string(), 303);
    redirects.insert(b"/d".to_string(), b"/4".to_string(), 307);
    redirects.insert(b"/e".to_string(), b"/5".to_string(), 308);
    assert!(redirects.length() == 5);
}

#[test]
fun test_all_valid_status_codes_via_fill() {
    let _redirects = redirects::filled(
        vector[
            b"/a".to_string(),
            b"/b".to_string(),
            b"/c".to_string(),
            b"/d".to_string(),
            b"/e".to_string(),
        ],
        vector[
            b"/1".to_string(),
            b"/2".to_string(),
            b"/3".to_string(),
            b"/4".to_string(),
            b"/5".to_string(),
        ],
        vector[301, 302, 303, 307, 308],
    );
}

#[test]
#[expected_failure(abort_code = EInvalidRedirectStatusCode)]
fun test_invalid_status_code_200() {
    redirects::filled(
        vector[b"/a".to_string()],
        vector[b"/b".to_string()],
        vector[200],
    );
}

#[test]
#[expected_failure(abort_code = EInvalidRedirectStatusCode)]
fun test_invalid_status_code_300() {
    redirects::filled(
        vector[b"/a".to_string()],
        vector[b"/b".to_string()],
        vector[300],
    );
}

#[test]
#[expected_failure(abort_code = EInvalidRedirectStatusCode)]
fun test_invalid_status_code_304() {
    redirects::filled(
        vector[b"/a".to_string()],
        vector[b"/b".to_string()],
        vector[304],
    );
}

#[test]
#[expected_failure(abort_code = EInvalidRedirectStatusCode)]
fun test_invalid_status_code_404() {
    redirects::filled(
        vector[b"/a".to_string()],
        vector[b"/b".to_string()],
        vector[404],
    );
}

#[test]
#[expected_failure(abort_code = EInvalidRedirectStatusCode)]
fun test_insert_invalid_status_code() {
    let mut redirects = redirects::empty();
    redirects.insert(b"/a".to_string(), b"/b".to_string(), 999);
}

// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

/// Redirect rules for Walrus Sites.
///
/// Each redirect maps a source path to a destination (the `Location` header value)
/// and an HTTP status code (301, 302, 303, 307, or 308).
module walrus_site::redirects;

use std::string::String;
use sui::vec_map::{Self, VecMap};

const EInvalidRedirectStatusCode: u64 = 0;
const ELocationsLength: u64 = 1;
const EStatusCodesLengths: u64 = 2;

public(package) macro fun redirects_field(): vector<u8> {
    b"redirects"
}

/// A single redirect entry: the destination URL and the HTTP status code.
public struct Redirect has drop, store {
    location: String,
    status_code: u16,
}

/// The collection of redirect rules for a site, stored as a dynamic field.
public struct Redirects has drop, store {
    redirect_list: VecMap<String, Redirect>,
}

/// Creates a new `Redirects` object.
public fun empty(): Redirects {
    Redirects { redirect_list: vec_map::empty() }
}

/// Creates a new `Redirects` object populated with the given entries.
///
/// Entries are inserted in reverse order (via `pop_back`), so the caller should
/// pass the vectors in reverse of the desired VecMap order, if ordering is important.
///
/// Aborts if the vectors have different lengths, any status code is invalid,
/// or any path is duplicated.
public fun filled(
    paths: vector<String>,
    locations: vector<String>,
    status_codes: vector<u16>,
): Redirects {
    let mut redirects = Redirects { redirect_list: vec_map::empty() };
    redirects.fill(paths, locations, status_codes);
    redirects
}

/// Populates an existing `Redirects` object with the given entries.
///
/// Entries are inserted in reverse order (via `pop_back`), so the caller should
/// pass the vectors in reverse of the desired VecMap order, if ordering is important.
///
/// Aborts if the vectors have different lengths, any status code is invalid,
/// or any path is duplicated.
public fun fill(
    self: &mut Redirects,
    mut paths: vector<String>,
    mut locations: vector<String>,
    mut status_codes: vector<u16>,
) {
    let Redirects { redirect_list } = self;
    let len = paths.length();
    assert!(len == locations.length(), ELocationsLength);
    assert!(len == status_codes.length(), EStatusCodesLengths);
    len.do!(|_| {
        let status_code = status_codes.pop_back();
        assert_redirect_status_code!(status_code);
        redirect_list.insert(
            paths.pop_back(),
            Redirect { location: locations.pop_back(), status_code },
        );
    });
}

/// Inserts a redirect into the `Redirects` object.
///
/// Aborts if the path already exists or the status code is invalid.
public fun insert(redirects: &mut Redirects, redirect: String, location: String, status_code: u16) {
    assert_redirect_status_code!(status_code);
    redirects.redirect_list.insert(redirect, Redirect { location, status_code });
}

/// Removes a redirect from the `Redirects` object.
public fun remove(redirects: &mut Redirects, redirect: &String): (String, String, u16) {
    let (path, Redirect { location, status_code }) = redirects.redirect_list.remove(redirect);
    (path, location, status_code)
}

/// Returns the number of redirects.
public fun length(self: &Redirects): u64 {
    self.redirect_list.length()
}

/// Returns the redirect entry for a given path.
///
/// Aborts if the path does not exist.
public fun get(self: &Redirects, path: &String): (&String, u16) {
    let Redirect { location, status_code } = self.redirect_list.get(path);
    (location, *status_code)
}

macro fun assert_redirect_status_code($status_code: u16) {
    let status_code = $status_code;
    assert!(
        status_code == 301 ||
        status_code == 302 ||
        status_code == 303 ||
        status_code == 307 ||
        status_code == 308,
        EInvalidRedirectStatusCode,
    );
}

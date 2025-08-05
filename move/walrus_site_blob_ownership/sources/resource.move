module walrus_site::resource;

use std::string::String;
use sui::vec_map::{Self, VecMap};

/// An insertion of route was attempted, but the related resource does not exist.
const ERangeStartGreaterThanRangeEnd: u64 = 0;
const EStartAndEndRangeAreNone: u64 = 1;

/// A resource in a site.
public struct Resource has drop, store {
    path: String,
    // Response, Representation and Payload headers
    // regarding the contents of the resource.
    headers: VecMap<String, String>,
    // The walrus blob id containing the bytes for this resource.
    blob_id: u256,
    // Contains the hash of the contents of the blob
    // to verify its integrity.
    blob_hash: u256,
    // Defines the byte range of the resource contents
    // in the case where multiple resources are stored
    // in the same blob. This way, each resource will
    // be parsed using its' byte range in the blob.
    range: Option<Range>,
}

/// Quilt range
public struct Range has drop, store {
    start: Option<u16>, // inclusive lower bound
    end: Option<u16>, // exclusive upper bound
}

/// Optionally creates a new Range object.
public fun new_range_option(range_start: Option<u16>, range_end: Option<u16>): Option<Range> {
    if (range_start.is_none() && range_end.is_none()) {
        return option::none<Range>()
    };
    option::some(new_range(range_start, range_end))
}

/// Creates a new Range object.
///
/// aborts if both range_start and range_end are none.
public fun new_range(range_start: Option<u16>, range_end: Option<u16>): Range {
    let start_is_defined = range_start.is_some();
    let end_is_defined = range_end.is_some();

    // At least one of the range bounds should be defined.
    assert!(start_is_defined || end_is_defined, EStartAndEndRangeAreNone);

    // If both range bounds are defined, the upper bound should be greater than the lower.
    if (start_is_defined && end_is_defined) {
        let start = option::borrow(&range_start);
        let end = option::borrow(&range_end);
        assert!(*end > *start, ERangeStartGreaterThanRangeEnd);
    };

    Range {
        start: range_start,
        end: range_end,
    }
}

/// Creates a new resource.
public fun new_resource(
    path: String,
    blob_id: u256,
    blob_hash: u256,
    range: Option<Range>,
): Resource {
    Resource {
        path,
        headers: vec_map::empty(),
        blob_id,
        blob_hash,
        range,
    }
}

/// Adds a header to the Resource's headers vector.
public fun add_header(resource: &mut Resource, name: String, value: String) {
    resource.headers.insert(name, value);
}

public fun path_mut(self: &mut Resource): &mut String {
    &mut self.path
}

public fun path(self: &Resource): &String {
    &self.path
}

public fun blob_id(self: &Resource): u256 {
    self.blob_id
}

// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{collections::BTreeMap, num::NonZeroU16, path::PathBuf};

use bytesize::ByteSize;
use move_core_types::u256::U256;

use super::{ResourceData, ResourceManager, MAX_IDENTIFIER_SIZE};
use crate::{
    config::Walrus,
    site_config::WSResources,
    types::{HttpHeaders, VecMap},
    walrus::command::QuiltBlobInput,
};

#[test]
fn test_derive_http_headers() {
    let test_paths = vec![
        // This is the longest path. So `/foo/bar/baz/*.svg` would persist over `*.svg`.
        ("/foo/bar/baz/image.svg", "etag"),
        // This will only match `*.svg`.
        (
            "/very_long_name_that_should_not_be_matched.svg",
            "cache-control",
        ),
    ];
    let ws_resources = mock_ws_resources();
    for (path, expected) in test_paths {
        let result = ResourceManager::derive_http_headers(&ws_resources, path);
        assert_eq!(result.len(), 1);
        assert!(result.contains_key(expected));
    }
}

/// Helper function for testing the `derive_http_headers` method.
fn mock_ws_resources() -> Option<WSResources> {
    let headers_json = r#"{
                    "/*.svg": {
                        "cache-control": "public, max-age=86400"
                    },
                    "/foo/bar/baz/*.svg": {
                        "etag": "\"abc123\""
                    }
                }"#;
    let headers: BTreeMap<String, HttpHeaders> = serde_json::from_str(headers_json).unwrap();

    Some(WSResources {
        headers: Some(headers),
        routes: None,
        metadata: None,
        site_name: None,
        object_id: None,
        ignore: None,
    })
}

/// Helper function to create a mock resource for testing.
fn create_mock_resource(
    file_name: String,
    file_size: usize,
    index: u64,
) -> (ResourceData, QuiltBlobInput) {
    let resource_data = ResourceData {
        unencoded_size: file_size,
        full_path: PathBuf::from(format!("/test/{}", file_name)),
        resource_path: format!("/{}", file_name),
        headers: HttpHeaders(VecMap::new()),
        blob_hash: U256::from(index),
    };

    let quilt_input = QuiltBlobInput {
        path: PathBuf::from(format!("/test/{}", file_name)),
        identifier: Some(format!("/{}", file_name)),
        tags: BTreeMap::new(),
    };

    (resource_data, quilt_input)
}

#[test]
fn test_large_file_below_theoretical_limit_placed_in_own_chunk() {
    // Set up test parameters
    let n_shards = NonZeroU16::new(100).unwrap();
    let max_available_columns = Walrus::max_slots_in_quilt(n_shards) as usize;
    let max_theoretical_quilt_size = Walrus::max_slot_size(n_shards) * max_available_columns;

    // Set max_quilt_size to be smaller than theoretical limit
    let max_quilt_size = ByteSize((max_theoretical_quilt_size / 2) as u64);
    let effective_quilt_size = max_quilt_size.as_u64() as usize;

    // Create a file that exceeds effective_quilt_size but is below max_theoretical_quilt_size
    const FIXED_OVERHEAD: usize = 8;
    let file_size = effective_quilt_size + 1000;

    // Ensure our file is in the correct range
    assert!(file_size + MAX_IDENTIFIER_SIZE + FIXED_OVERHEAD > effective_quilt_size);
    assert!(file_size + MAX_IDENTIFIER_SIZE + FIXED_OVERHEAD < max_theoretical_quilt_size);

    // Create mock resource
    let resource = create_mock_resource("large_file.bin".to_string(), file_size, 0);

    // Create a mock ResourceManager
    let resource_manager = ResourceManager {
        walrus: Walrus::new("walrus".to_string(), 1000000, None, None, None, None),
        ws_resources: None,
        ws_resources_path: None,
        n_shards,
    };

    // Call quilts_chunkify
    let result = resource_manager.quilts_chunkify(vec![resource], max_quilt_size);

    // The function should not error, and should place the file in its own chunk
    let chunks = result.expect("Should not fail for file below theoretical limit");
    assert_eq!(chunks.len(), 1, "Should have exactly one chunk");
    assert_eq!(chunks[0].len(), 1, "Chunk should contain exactly one file");
}

#[test]
fn test_large_file_among_small_files_creates_correct_chunks() {
    // Set up test parameters
    let n_shards = NonZeroU16::new(100).unwrap();
    let max_available_columns = Walrus::max_slots_in_quilt(n_shards) as usize;
    let max_theoretical_quilt_size = Walrus::max_slot_size(n_shards) * max_available_columns;

    // Set max_quilt_size to be smaller than theoretical limit
    let max_quilt_size = ByteSize((max_theoretical_quilt_size / 2) as u64);
    let effective_quilt_size = max_quilt_size.as_u64() as usize;

    const FIXED_OVERHEAD: usize = 8;

    // Small files that fit easily in a chunk
    let small_file_size = 1000;

    // Large file exceeds effective_quilt_size but is below theoretical limit
    let large_file_size = effective_quilt_size + 1000;

    // Verify our assumptions
    assert!(large_file_size + MAX_IDENTIFIER_SIZE + FIXED_OVERHEAD > effective_quilt_size);
    assert!(large_file_size + MAX_IDENTIFIER_SIZE + FIXED_OVERHEAD < max_theoretical_quilt_size);

    // Create 21 files: file at index 4 is large, rest are small
    let mut files = vec![];

    // Files 0-3: small files
    for i in 0..4 {
        files.push(create_mock_resource(
            format!("small_file_{}.bin", i),
            small_file_size,
            i,
        ));
    }

    // File 4: large file
    files.push(create_mock_resource(
        "large_file_4.bin".to_string(),
        large_file_size,
        4,
    ));

    // Files 5-20: small files
    for i in 5..21 {
        files.push(create_mock_resource(
            format!("small_file_{}.bin", i),
            small_file_size,
            i,
        ));
    }

    // Create a mock ResourceManager
    let resource_manager = ResourceManager {
        walrus: Walrus::new("walrus".to_string(), 1000000, None, None, None, None),
        ws_resources: None,
        ws_resources_path: None,
        n_shards,
    };

    // Call quilts_chunkify
    let result = resource_manager.quilts_chunkify(files, max_quilt_size);

    // Expected behavior:
    // - The large file should be alone in the first chunk
    // - The remaining small files should be packed in subsequent chunks
    let chunks = result.expect("Should not fail");

    // Verify the large file is alone in chunk[0]
    assert_eq!(
        chunks[0].len(),
        1,
        "First chunk should contain only the large file"
    );
    assert!(
        chunks[0][0].0.resource_path.contains("large_file_4"),
        "First chunk should contain the large file"
    );

    // Verify the next chunk has multiple small files
    assert!(
        chunks.len() > 1,
        "Should have at least one more chunk with small files"
    );
    assert!(
        chunks[1].len() > 4,
        "Second chunk should contain multiple small files (more than 4)"
    );
}

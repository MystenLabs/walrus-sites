// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;

use super::{HttpHeaders, ResourceManager};
use crate::site_config::WSResources;

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

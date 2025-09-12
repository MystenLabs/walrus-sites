// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use site_builder::{
    site::config::WSResources,
    util::{is_ignored, is_pattern_match},
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
    assert!(is_ignored(&ws_resources.ignore, "/foo/nested/bar.txt"));
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
        PatternMatchTestCase {
            pattern: "[invalid",
            path: "/file",
            expected: false,
        },
    ];
    for t in tests {
        assert_eq!(is_pattern_match(t.pattern, t.path), t.expected);
    }
}

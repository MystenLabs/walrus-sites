// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::path::{Path, PathBuf};

use super::DirNode;

#[test]
fn test_dirnode_to_html() {
    let dir = DirNode {
        contents: vec![
            PathBuf::from("/my/sub/path"),
            PathBuf::from("/my/sub/another"),
        ],
        path: PathBuf::from("/my/sub"),
    };
    let expected = r#"<!DOCTYPE html>
<html>
<head>
<title>Directory listing for /sub</title>
</head>
<body>
<h1>Directory listing for /sub</h1>
<hr>
<ul>
<li><a href="/sub/another">another</a></li>
<li><a href="/sub/path">path</a></li>
</ul>
<hr>
</body>
</html>"#;
    assert_eq!(dir.to_html(Path::new("/my/")).unwrap(), expected);
}

#[test]
fn test_dirnode_to_html_nested_path() {
    // Test with deeper nesting
    let dir = DirNode {
        contents: vec![PathBuf::from("/root/site/a/b/c/file.txt")],
        path: PathBuf::from("/root/site/a/b/c"),
    };
    let html = dir.to_html(Path::new("/root/site")).unwrap();
    assert!(html.contains("Directory listing for /a/b/c"));
    assert!(html.contains(r#"<a href="/a/b/c/file.txt">file.txt</a>"#));
}

#[test]
fn test_dirnode_to_html_empty_directory() {
    let dir = DirNode {
        contents: vec![],
        path: PathBuf::from("/root/site/empty"),
    };
    let html = dir.to_html(Path::new("/root/site")).unwrap();
    assert!(html.contains("Directory listing for /empty"));
    assert!(html.contains("<ul>\n\n</ul>"));
}

#[test]
fn test_path_to_html_generates_correct_links() {
    // Test that path_to_html generates correct links for files
    let path = Path::new("/root/site/file.txt");
    let root = Path::new("/root/site");
    let html = DirNode::path_to_html(path, root).unwrap();
    assert_eq!(html, r#"<a href="/file.txt">file.txt</a>"#);
}

#[test]
fn test_path_to_html_nested_file() {
    // Test path_to_html with a nested file path
    let path = Path::new("/root/site/subdir/nested/file.html");
    let root = Path::new("/root/site");
    let html = DirNode::path_to_html(path, root).unwrap();
    assert_eq!(html, r#"<a href="/subdir/nested/file.html">file.html</a>"#);
}

#[test]
fn test_path_to_html_special_characters_in_filename() {
    // Test with special characters that are valid in filenames
    let path = Path::new("/root/site/file-with_special.chars.txt");
    let root = Path::new("/root/site");
    let html = DirNode::path_to_html(path, root).unwrap();
    assert_eq!(
        html,
        r#"<a href="/file-with_special.chars.txt">file-with_special.chars.txt</a>"#
    );
}

#[cfg(not(windows))]
#[test]
fn test_path_to_html_unix_literal_backslash_in_filename() {
    // On Unix, backslash is a valid character in filenames.
    // Test that path_to_html preserves literal backslashes.
    let path = Path::new("/root/site/file\\with\\backslash.txt");
    let root = Path::new("/root/site");
    let html = DirNode::path_to_html(path, root).unwrap();
    // The backslash should be preserved in both the href and the display name
    assert_eq!(
        html,
        r#"<a href="/file\with\backslash.txt">file\with\backslash.txt</a>"#
    );
}

#[cfg(windows)]
#[test]
fn test_path_to_html_windows_backslash_normalization() {
    // On Windows, backslash is the path separator.
    // Test that path_to_html converts backslashes to forward slashes in URLs.
    let path = Path::new(r"C:\root\site\subdir\file.txt");
    let root = Path::new(r"C:\root\site");
    let html = DirNode::path_to_html(path, root).unwrap();
    // Backslashes should be converted to forward slashes in the href
    assert_eq!(html, r#"<a href="/subdir/file.txt">file.txt</a>"#);
}

#[test]
fn test_dirnode_new() {
    let path = PathBuf::from("/test/path");
    let node = DirNode::new(path.clone());
    assert!(node.contents.is_empty());
    assert_eq!(node.path, path);
}

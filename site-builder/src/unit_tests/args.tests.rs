// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;

use clap::Parser;

use super::{Args, Commands, ResourcePaths};

#[test]
fn test_max_quilt_size() {
    // Test parsing "10MiB" (binary megabytes)
    let args = Args::try_parse_from([
        "site-builder",
        "deploy",
        "/some/path",
        "--epochs",
        "5",
        "--max-quilt-size",
        "10MiB",
    ]);
    assert!(args.is_ok());

    if let Commands::Deploy {
        publish_options, ..
    } = &args.unwrap().command
    {
        let size_bytes = publish_options.walrus_options.max_quilt_size.as_u64();
        assert_eq!(size_bytes, 10 * 1024 * 1024); // 10MiB in bytes
    } else {
        panic!("Expected Deploy command");
    }

    // Test parsing "10MB" (decimal megabytes)
    let args = Args::try_parse_from([
        "site-builder",
        "deploy",
        "/some/path",
        "--epochs",
        "5",
        "--max-quilt-size",
        "10MB",
    ]);
    assert!(args.is_ok());

    if let Commands::Deploy {
        publish_options, ..
    } = &args.unwrap().command
    {
        let size_bytes = publish_options.walrus_options.max_quilt_size.as_u64();
        assert_eq!(size_bytes, 10_000_000); // 10MB in bytes (decimal)
    } else {
        panic!("Expected Deploy command");
    }

    // Test parsing "5GiB" (binary gigabytes)
    let args = Args::try_parse_from([
        "site-builder",
        "deploy",
        "/some/path",
        "--epochs",
        "5",
        "--max-quilt-size",
        "5GiB",
    ]);
    assert!(args.is_ok());

    if let Commands::Deploy {
        publish_options, ..
    } = &args.unwrap().command
    {
        let size_bytes = publish_options.walrus_options.max_quilt_size.as_u64();
        assert_eq!(size_bytes, 5 * 1024 * 1024 * 1024); // 5GiB in bytes
    } else {
        panic!("Expected Deploy command");
    }

    // Test parsing "512KiB" (binary kilobytes)
    let args = Args::try_parse_from([
        "site-builder",
        "deploy",
        "/some/path",
        "--epochs",
        "5",
        "--max-quilt-size",
        "512KiB",
    ]);
    assert!(args.is_ok());

    if let Commands::Deploy {
        publish_options, ..
    } = &args.unwrap().command
    {
        let size_bytes = publish_options.walrus_options.max_quilt_size.as_u64();
        assert_eq!(size_bytes, 512 * 1024); // 512KiB in bytes
    } else {
        panic!("Expected Deploy command");
    }

    // Test parsing plain bytes number (backward compatibility)
    let args = Args::try_parse_from([
        "site-builder",
        "deploy",
        "/some/path",
        "--epochs",
        "5",
        "--max-quilt-size",
        "1048576",
    ]);
    assert!(args.is_ok());

    if let Commands::Deploy {
        publish_options, ..
    } = &args.unwrap().command
    {
        let size_bytes = publish_options.walrus_options.max_quilt_size.as_u64();
        assert_eq!(size_bytes, 1048576); // 1MiB in bytes
    } else {
        panic!("Expected Deploy command");
    }
}

#[test]
fn test_resource_arg_basic() {
    // Test basic parsing without escapes
    let result = "index.html:/index.html".parse::<ResourcePaths>();
    assert!(result.is_ok());
    let ResourcePaths {
        file_path,
        url_path,
    } = result.unwrap();
    assert_eq!(file_path, PathBuf::from("index.html"));
    assert_eq!(url_path, "/index.html");
}

#[test]
fn test_resource_arg_with_whitespace() {
    // Test file name with whitespace
    let result = "my file.html:/my-file.html".parse::<ResourcePaths>();
    assert!(result.is_ok());
    let ResourcePaths {
        file_path,
        url_path,
    } = result.unwrap();
    assert_eq!(file_path, PathBuf::from("my file.html"));
    assert_eq!(url_path, "/my-file.html");
}

#[test]
fn test_resource_arg_with_escaped_colon() {
    // Test file name with escaped colon
    let result = r"file\:name.html:/site/path.html".parse::<ResourcePaths>();
    assert!(result.is_ok());
    let ResourcePaths {
        file_path,
        url_path,
    } = result.unwrap();
    assert_eq!(file_path, PathBuf::from("file:name.html"));
    assert_eq!(url_path, "/site/path.html");
}

#[test]
fn test_resource_arg_file_ending_with_colon() {
    // Test file ending with colon (escaped)
    let result = r"file\::/site/path.html".parse::<ResourcePaths>();
    assert!(result.is_ok());
    let ResourcePaths {
        file_path,
        url_path,
    } = result.unwrap();
    assert_eq!(file_path, PathBuf::from("file:"));
    assert_eq!(url_path, "/site/path.html");
}

#[test]
fn test_resource_arg_multiple_escaped_colons() {
    // Test multiple escaped colons in filename
    let result = r"file\:name\:test.html:/site/path.html".parse::<ResourcePaths>();
    assert!(result.is_ok());
    let ResourcePaths {
        file_path,
        url_path,
    } = result.unwrap();
    assert_eq!(file_path, PathBuf::from("file:name:test.html"));
    assert_eq!(url_path, "/site/path.html");
}

#[test]
fn test_resource_arg_site_path_with_escaped_colon() {
    // Test site path with escaped colon
    let result = r"file.html:/site\:path.html".parse::<ResourcePaths>();
    assert!(result.is_ok());
    let ResourcePaths {
        file_path,
        url_path,
    } = result.unwrap();
    assert_eq!(file_path, PathBuf::from("file.html"));
    assert_eq!(url_path, "/site:path.html");
}

#[test]
fn test_resource_arg_invalid_no_colon() {
    // Test invalid format without colon
    let result = "file.html".parse::<ResourcePaths>();
    assert!(result.is_err());
}

#[test]
fn test_resource_arg_invalid_too_many_colons() {
    // Test invalid format with too many unescaped colons
    let result = "file.html:/site/path.html:/extra".parse::<ResourcePaths>();
    assert!(result.is_err());
}

#[test]
fn test_resource_arg_complex_path() {
    // Test with complex paths
    let result = "../path/to/file.html:/assets/styles/main.css".parse::<ResourcePaths>();
    assert!(result.is_ok());
    let ResourcePaths {
        file_path,
        url_path,
    } = result.unwrap();
    assert_eq!(file_path, PathBuf::from("../path/to/file.html"));
    assert_eq!(url_path, "/assets/styles/main.css");
}

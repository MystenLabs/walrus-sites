// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;

use crate::args::{Args, Commands};

#[test]
fn test_max_quilt_size() {
    // Test parsing "10MiB" (binary megabytes)
    let args = Args::try_parse_from([
        "site-builder",
        "publish-quilts",
        "/some/path",
        "--epochs",
        "5",
        "--max-quilt-size",
        "10MiB",
    ]);
    assert!(args.is_ok());

    if let Commands::Publish {
        publish_options, ..
    } = &args.unwrap().command
    {
        let size_bytes = publish_options.walrus_options.max_quilt_size.as_u64();
        assert_eq!(size_bytes, 10 * 1024 * 1024); // 10MiB in bytes
    } else {
        panic!("Expected PublishQuilts command");
    }

    // Test parsing "10MB" (decimal megabytes)
    let args = Args::try_parse_from([
        "site-builder",
        "publish-quilts",
        "/some/path",
        "--epochs",
        "5",
        "--max-quilt-size",
        "10MB",
    ]);
    assert!(args.is_ok());

    if let Commands::Publish {
        publish_options, ..
    } = &args.unwrap().command
    {
        let size_bytes = publish_options.walrus_options.max_quilt_size.as_u64();
        assert_eq!(size_bytes, 10_000_000); // 10MB in bytes (decimal)
    } else {
        panic!("Expected PublishQuilts command");
    }

    // Test parsing "5GiB" (binary gigabytes)
    let args = Args::try_parse_from([
        "site-builder",
        "publish-quilts",
        "/some/path",
        "--epochs",
        "5",
        "--max-quilt-size",
        "5GiB",
    ]);
    assert!(args.is_ok());

    if let Commands::Publish {
        publish_options, ..
    } = &args.unwrap().command
    {
        let size_bytes = publish_options.walrus_options.max_quilt_size.as_u64();
        assert_eq!(size_bytes, 5 * 1024 * 1024 * 1024); // 5GiB in bytes
    } else {
        panic!("Expected PublishQuilts command");
    }

    // Test parsing "512KiB" (binary kilobytes)
    let args = Args::try_parse_from([
        "site-builder",
        "publish-quilts",
        "/some/path",
        "--epochs",
        "5",
        "--max-quilt-size",
        "512KiB",
    ]);
    assert!(args.is_ok());

    if let Commands::Publish {
        publish_options, ..
    } = &args.unwrap().command
    {
        let size_bytes = publish_options.walrus_options.max_quilt_size.as_u64();
        assert_eq!(size_bytes, 512 * 1024); // 512KiB in bytes
    } else {
        panic!("Expected PublishQuilts command");
    }

    // Test parsing plain bytes number (backward compatibility)
    let args = Args::try_parse_from([
        "site-builder",
        "publish-quilts",
        "/some/path",
        "--epochs",
        "5",
        "--max-quilt-size",
        "1048576",
    ]);
    assert!(args.is_ok());

    if let Commands::Publish {
        publish_options, ..
    } = &args.unwrap().command
    {
        let size_bytes = publish_options.walrus_options.max_quilt_size.as_u64();
        assert_eq!(size_bytes, 1048576); // 1MiB in bytes
    } else {
        panic!("Expected PublishQuilts command");
    }
}

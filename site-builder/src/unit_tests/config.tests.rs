// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::io::Write;

use tempfile::NamedTempFile;

use super::Config;

fn write_temp_config(content: &str) -> NamedTempFile {
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f
}

#[tokio::test]
async fn singleton_with_package_succeeds() {
    let f = write_temp_config(
        r#"
package: "0x1234"
"#,
    );
    let (config, context) = Config::load_from_multi_config(f.path(), None)
        .await
        .unwrap();
    assert!(config.package.is_some());
    assert!(context.is_none());
}

#[tokio::test]
async fn singleton_without_package_fails() {
    let f = write_temp_config(
        r#"
portal: "wal.app"
"#,
    );
    let err = Config::load_from_multi_config(f.path(), None)
        .await
        .unwrap_err();
    assert!(
        err.to_string().contains("package"),
        "error should mention `package`: {err}"
    );
    assert!(
        err.to_string().contains("singleton"),
        "error should mention singleton: {err}"
    );
}

#[tokio::test]
async fn singleton_with_context_specified_fails() {
    let f = write_temp_config(
        r#"
package: "0x1234"
"#,
    );
    let err = Config::load_from_multi_config(f.path(), Some("testnet"))
        .await
        .unwrap_err();
    assert!(
        err.to_string().contains("cannot specify context"),
        "error should mention context conflict: {err}"
    );
}

#[tokio::test]
async fn multi_config_with_package_succeeds() {
    let f = write_temp_config(
        r#"
contexts:
  testnet:
    package: "0xabc"
default_context: testnet
"#,
    );
    let (config, context) = Config::load_from_multi_config(f.path(), None)
        .await
        .unwrap();
    assert!(config.package.is_some());
    assert_eq!(context.as_deref(), Some("testnet"));
}

#[tokio::test]
async fn multi_config_explicit_context_overrides_default() {
    let f = write_temp_config(
        r#"
contexts:
  testnet:
    package: "0xabc"
  mainnet:
    package: "0xdef"
default_context: testnet
"#,
    );
    let (config, context) = Config::load_from_multi_config(f.path(), Some("mainnet"))
        .await
        .unwrap();
    assert_eq!(context.as_deref(), Some("mainnet"));
    assert_eq!(
        config.package.unwrap().to_string(),
        "0x0000000000000000000000000000000000000000000000000000000000000def"
    );
}

#[tokio::test]
async fn multi_config_missing_context_fails() {
    let f = write_temp_config(
        r#"
contexts:
  testnet:
    package: "0xabc"
default_context: testnet
"#,
    );
    let err = Config::load_from_multi_config(f.path(), Some("devnet"))
        .await
        .unwrap_err();
    assert!(
        err.to_string().contains("devnet"),
        "error should mention missing context: {err}"
    );
}

#[tokio::test]
async fn multi_config_without_package_resolves_via_mvr() {
    let f = write_temp_config(
        r#"
contexts:
  testnet:
    portal: "wal.app"
default_context: testnet
"#,
    );
    // MVR resolution is attempted for the "testnet" context. If network is available,
    // it succeeds and populates `package`. If not, the error comes from MVR, not from
    // a "package required" check.
    match Config::load_from_multi_config(f.path(), None).await {
        Ok((config, _)) => {
            assert!(
                config.package.is_some(),
                "MVR should have resolved the package"
            );
        }
        Err(err) => {
            let msg = err.to_string();
            assert!(
                !msg.contains("singleton"),
                "multi-config without package should attempt MVR, not fail like singleton: {msg}"
            );
            assert!(
                msg.contains("MVR"),
                "error should mention MVR resolution failure: {msg}"
            );
        }
    }
}

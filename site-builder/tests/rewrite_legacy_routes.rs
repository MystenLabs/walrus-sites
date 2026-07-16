// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! End-to-end tests for deploy-time route-pattern validation and the
//! `--rewrite-legacy-routes` flag, against a local test cluster.

use std::{collections::BTreeMap, fs::File, path::Path};

use site_builder::{
    args::{Commands, EpochCountOrMax},
    site_config::WSResources,
    types::{Redirect, Redirects, Routes, VecMap},
};

#[allow(dead_code)]
mod localnode;
use localnode::{
    args_builder::{ArgsBuilder, PublishOptionsBuilder},
    TestSetup,
};

#[allow(dead_code)]
mod helpers;
use helpers::create_test_site;

/// Writes a `ws-resources.json` with legacy-spelled routes (both rewrite
/// arms: a bare `*` and a trailing `/*`), an untouched exact route, and one
/// glob redirect, into the site directory.
fn write_legacy_ws_resources(directory: &Path) -> anyhow::Result<()> {
    let ws_resources = WSResources {
        routes: Some(Routes(VecMap(BTreeMap::from([
            ("*".to_owned(), "/file_0.html".to_owned()),
            ("/docs/*".to_owned(), "/file_1.html".to_owned()),
            ("/exact".to_owned(), "/file_0.html".to_owned()),
        ])))),
        redirects: Some(Redirects(VecMap(BTreeMap::from([(
            "/redirects/**/*".to_owned(),
            Redirect {
                location: "/file_0.html".to_owned(),
                status_code: 302,
            },
        )])))),
        ..Default::default()
    };
    serde_json::to_writer_pretty(
        File::create(directory.join("ws-resources.json"))?,
        &ws_resources,
    )?;
    Ok(())
}

fn read_ws_resources(directory: &Path) -> anyhow::Result<WSResources> {
    Ok(serde_json::from_reader(File::open(
        directory.join("ws-resources.json"),
    )?)?)
}

fn route_keys(routes: &Routes) -> Vec<&String> {
    routes.0 .0.keys().collect()
}

/// The full life of `--rewrite-legacy-routes`:
/// 1. publish with the flag: on-chain routes carry the glob spellings, while
///    `ws-resources.json` keeps the author's legacy spellings (in-memory-only
///    rewrite) and gains the new `object_id`;
/// 2. redirects are never rewritten;
/// 3. an update with the flag is idempotent (same stored spellings);
/// 4. an update WITHOUT the flag reverts the stored spellings to the raw
///    legacy ones — the documented ratchet.
#[tokio::test]
#[ignore]
async fn rewrite_legacy_routes_publish_update_revert() -> anyhow::Result<()> {
    let cluster = TestSetup::start_local_test_cluster(None).await?;
    let site_dir = create_test_site(2)?;
    let directory = site_dir.path().to_path_buf();
    write_legacy_ws_resources(&directory)?;

    // Publish with the flag on.
    let args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Publish {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory.clone())
                .with_epoch_count_or_max(EpochCountOrMax::Epochs(1_u32.try_into().unwrap()))
                .with_rewrite_legacy_routes(true)
                .build()?,
            site_name: None,
        })
        .build()?;
    site_builder::run(args).await?;

    let site_id = *cluster.last_site_created().await?.id.object_id();
    let routes = cluster
        .site_routes(site_id)
        .await?
        .expect("routes should exist on chain");
    assert_eq!(
        route_keys(&routes),
        ["/**", "/docs/**/*", "/exact"],
        "on-chain routes must use the rewritten glob spellings"
    );
    println!("✓ published with rewritten routes");

    // The rewrite is in-memory only: the file keeps the author's spellings,
    // while persist added the new object_id.
    let on_disk = read_ws_resources(&directory)?;
    assert_eq!(
        route_keys(on_disk.routes.as_ref().expect("routes on disk")),
        ["*", "/docs/*", "/exact"],
        "ws-resources.json must keep the legacy spellings"
    );
    assert_eq!(on_disk.object_id, Some(site_id));
    println!("✓ ws-resources.json kept the legacy spellings");

    // Redirects are never rewritten.
    let redirects = cluster
        .site_redirects(site_id)
        .await?
        .expect("redirects should exist on chain");
    assert!(redirects.0 .0.contains_key("/redirects/**/*"));
    assert_eq!(redirects.0.len(), 1);

    // Update with the flag on: rewrite-then-diff must be idempotent.
    let args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Update {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory.clone())
                .with_epoch_count_or_max(EpochCountOrMax::Max)
                .with_rewrite_legacy_routes(true)
                .build()?,
            object_id: site_id,
        })
        .build()?;
    site_builder::run(args).await?;
    let routes = cluster.site_routes(site_id).await?.expect("routes");
    assert_eq!(route_keys(&routes), ["/**", "/docs/**/*", "/exact"]);
    println!("✓ flag-on re-deploy is idempotent");

    // Update WITHOUT the flag: the raw legacy spellings diff against the
    // stored glob spellings and replace them — the documented revert ratchet.
    let args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Update {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory)
                .with_epoch_count_or_max(EpochCountOrMax::Max)
                .build()?,
            object_id: site_id,
        })
        .build()?;
    site_builder::run(args).await?;
    let routes = cluster.site_routes(site_id).await?.expect("routes");
    assert_eq!(
        route_keys(&routes),
        ["*", "/docs/*", "/exact"],
        "a flag-off re-deploy reverts to the legacy spellings"
    );
    println!("✓ flag-off re-deploy reverted the spellings");

    Ok(())
}

/// A structurally invalid glob pattern in a to-be-written set fails the
/// publish before anything is stored, naming the offender with the portal's
/// exact reason string; nothing is persisted on the error path.
#[tokio::test]
#[ignore]
async fn publish_rejects_invalid_glob_pattern() -> anyhow::Result<()> {
    let cluster = TestSetup::start_local_test_cluster(None).await?;
    let site_dir = create_test_site(1)?;
    let directory = site_dir.path().to_path_buf();

    let ws_resources = WSResources {
        routes: Some(Routes(VecMap(BTreeMap::from([(
            "a*b*".to_owned(),
            "/file_0.html".to_owned(),
        )])))),
        ..Default::default()
    };
    serde_json::to_writer_pretty(
        File::create(directory.join("ws-resources.json"))?,
        &ws_resources,
    )?;

    let args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Publish {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory.clone())
                .with_epoch_count_or_max(EpochCountOrMax::Epochs(1_u32.try_into().unwrap()))
                .build()?,
            site_name: None,
        })
        .build()?;
    let error = site_builder::run(args)
        .await
        .expect_err("publish must refuse the invalid pattern");
    let error_msg = format!("{error:?}");
    assert!(
        error_msg.contains("refusing to store them on-chain"),
        "unexpected error: {error_msg}"
    );
    assert!(
        error_msg.contains(r#"segment "a*b*" may use at most one '*', or be a whole-segment '**'"#),
        "unexpected error: {error_msg}"
    );
    println!("✓ invalid pattern refused with the pinned reason string");

    // The error path never persists: no object_id was written back.
    let on_disk = read_ws_resources(&directory)?;
    assert_eq!(on_disk.object_id, None);

    Ok(())
}

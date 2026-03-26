// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufWriter, Write},
};

use site_builder::{
    args::{Commands, EpochCountOrMax},
    site_config::WSResources,
    types::{Redirect, Redirects, Routes, VecMap},
};

#[allow(dead_code)]
mod helpers;
#[allow(dead_code)]
mod localnode;
use localnode::{
    args_builder::{ArgsBuilder, PublishOptionsBuilder},
    TestSetup,
};

fn make_redirects(entries: &[(&str, &str, u16)]) -> Redirects {
    Redirects(VecMap(
        entries
            .iter()
            .map(|(path, location, status_code)| {
                (
                    path.to_string(),
                    Redirect {
                        location: location.to_string(),
                        status_code: *status_code,
                    },
                )
            })
            .collect(),
    ))
}

fn write_ws_resources(
    dir: &std::path::Path,
    ws: &WSResources,
) -> anyhow::Result<std::path::PathBuf> {
    let path = dir.join("ws-resources.json");
    let file = File::create(&path)?;
    serde_json::to_writer_pretty(BufWriter::new(file), ws)?;
    Ok(path)
}

#[tokio::test]
#[ignore]
async fn publish_with_redirects() -> anyhow::Result<()> {
    let cluster = TestSetup::start_local_test_cluster(None).await?;

    let site_dir = tempfile::tempdir()?;
    for i in 0..3 {
        let mut f = File::create(site_dir.path().join(format!("page_{i}.html")))?;
        writeln!(f, "<html><body>Page {i}</body></html>")?;
    }

    let redirects = make_redirects(&[
        ("/old", "https://example.com/new", 301),
        ("/temp", "/other", 302),
    ]);
    let ws_dir = tempfile::tempdir()?;
    let ws_path = write_ws_resources(
        ws_dir.path(),
        &WSResources {
            redirects: Some(redirects),
            ..Default::default()
        },
    )?;

    let args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Publish {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(site_dir.path().to_owned())
                .with_ws_resources(Some(ws_path))
                .with_epoch_count_or_max(EpochCountOrMax::Max)
                .build()?,
            site_name: None,
        })
        .build()?;
    site_builder::run(args).await?;

    let site = cluster.last_site_created().await?;
    let site_id = *site.id.object_id();

    let resources = cluster.site_resources(site_id).await?;
    assert_eq!(resources.len(), 3);

    let on_chain_redirects = cluster.site_redirects(site_id).await?;
    let on_chain_redirects = on_chain_redirects.expect("redirects should exist on chain");
    assert_eq!(on_chain_redirects.0.len(), 2);

    let map: &BTreeMap<_, _> = &on_chain_redirects.0 .0;
    let old_redirect = &map["/old"];
    assert_eq!(old_redirect.location, "https://example.com/new");
    assert_eq!(old_redirect.status_code, 301);

    let temp_redirect = &map["/temp"];
    assert_eq!(temp_redirect.location, "/other");
    assert_eq!(temp_redirect.status_code, 302);

    Ok(())
}

#[tokio::test]
#[ignore]
async fn deploy_update_redirects() -> anyhow::Result<()> {
    let cluster = TestSetup::start_local_test_cluster(None).await?;

    let site_dir = tempfile::tempdir()?;
    let mut f = File::create(site_dir.path().join("index.html"))?;
    writeln!(f, "<html><body>Hello</body></html>")?;

    // Initial deploy with 2 redirects.
    let initial_redirects = make_redirects(&[("/a", "/target-a", 301), ("/b", "/target-b", 302)]);
    let ws_path = site_dir.path().join("ws-resources.json");
    serde_json::to_writer_pretty(
        BufWriter::new(File::create(&ws_path)?),
        &WSResources {
            redirects: Some(initial_redirects),
            ..Default::default()
        },
    )?;

    let deploy = |dir: std::path::PathBuf| {
        let config = cluster.sites_config_path().to_owned();
        async move {
            let args = ArgsBuilder::default()
                .with_config(Some(config))
                .with_command(Commands::Deploy {
                    publish_options: PublishOptionsBuilder::default()
                        .with_directory(dir)
                        .with_epoch_count_or_max(EpochCountOrMax::Max)
                        .build()
                        .unwrap(),
                    site_name: None,
                    object_id: None,
                })
                .build()
                .unwrap();
            site_builder::run(args).await
        }
    };

    deploy(site_dir.path().to_owned()).await?;

    let site = cluster.last_site_created().await?;
    let site_id = *site.id.object_id();

    // Verify initial redirects.
    let r = cluster.site_redirects(site_id).await?.unwrap();
    assert_eq!(r.0.len(), 2);
    assert_eq!(r.0 .0["/a"].status_code, 301);
    assert_eq!(r.0 .0["/b"].status_code, 302);

    // Update: change /a destination, add /c, remove /b.
    let updated_redirects =
        make_redirects(&[("/a", "/new-target-a", 301), ("/c", "/target-c", 307)]);
    serde_json::to_writer_pretty(
        BufWriter::new(File::create(&ws_path)?),
        &WSResources {
            redirects: Some(updated_redirects),
            object_id: Some(site_id),
            ..Default::default()
        },
    )?;

    deploy(site_dir.path().to_owned()).await?;

    // Verify: site ID unchanged, redirects updated.
    let updated_site = cluster.last_site_created().await?;
    assert_eq!(site_id, *updated_site.id.object_id());

    let r = cluster.site_redirects(site_id).await?.unwrap();
    assert_eq!(r.0.len(), 2);
    assert_eq!(r.0 .0["/a"].location, "/new-target-a");
    assert_eq!(r.0 .0["/c"].status_code, 307);
    assert!(!r.0 .0.contains_key("/b"));

    Ok(())
}

#[tokio::test]
#[ignore]
async fn deploy_remove_all_redirects() -> anyhow::Result<()> {
    let cluster = TestSetup::start_local_test_cluster(None).await?;

    let site_dir = tempfile::tempdir()?;
    let mut f = File::create(site_dir.path().join("index.html"))?;
    writeln!(f, "<html><body>Hello</body></html>")?;

    let ws_path = site_dir.path().join("ws-resources.json");

    // Deploy with redirects.
    serde_json::to_writer_pretty(
        BufWriter::new(File::create(&ws_path)?),
        &WSResources {
            redirects: Some(make_redirects(&[
                ("/old", "/new", 301),
                ("/temp", "/other", 308),
            ])),
            ..Default::default()
        },
    )?;

    let deploy = |dir: std::path::PathBuf, config: std::path::PathBuf| async move {
        let args = ArgsBuilder::default()
            .with_config(Some(config))
            .with_command(Commands::Deploy {
                publish_options: PublishOptionsBuilder::default()
                    .with_directory(dir)
                    .with_epoch_count_or_max(EpochCountOrMax::Max)
                    .build()
                    .unwrap(),
                site_name: None,
                object_id: None,
            })
            .build()
            .unwrap();
        site_builder::run(args).await
    };

    deploy(
        site_dir.path().to_owned(),
        cluster.sites_config_path().to_owned(),
    )
    .await?;

    let site_id = *cluster.last_site_created().await?.id.object_id();
    assert!(cluster.site_redirects(site_id).await?.is_some());

    // Deploy again without redirects.
    let ws: WSResources = serde_json::from_reader(File::open(&ws_path)?)?;
    serde_json::to_writer_pretty(
        BufWriter::new(File::create(&ws_path)?),
        &WSResources {
            redirects: None,
            object_id: ws.object_id,
            ..Default::default()
        },
    )?;

    deploy(
        site_dir.path().to_owned(),
        cluster.sites_config_path().to_owned(),
    )
    .await?;

    assert!(
        cluster.site_redirects(site_id).await?.is_none(),
        "redirects should be removed from chain"
    );

    Ok(())
}

#[tokio::test]
#[ignore]
async fn deploy_add_redirects_to_existing_site() -> anyhow::Result<()> {
    let cluster = TestSetup::start_local_test_cluster(None).await?;

    let site_dir = tempfile::tempdir()?;
    let mut f = File::create(site_dir.path().join("index.html"))?;
    writeln!(f, "<html><body>Hello</body></html>")?;

    let ws_path = site_dir.path().join("ws-resources.json");

    // Deploy without redirects.
    serde_json::to_writer_pretty(
        BufWriter::new(File::create(&ws_path)?),
        &WSResources::default(),
    )?;

    let deploy = |dir: std::path::PathBuf, config: std::path::PathBuf| async move {
        let args = ArgsBuilder::default()
            .with_config(Some(config))
            .with_command(Commands::Deploy {
                publish_options: PublishOptionsBuilder::default()
                    .with_directory(dir)
                    .with_epoch_count_or_max(EpochCountOrMax::Max)
                    .build()
                    .unwrap(),
                site_name: None,
                object_id: None,
            })
            .build()
            .unwrap();
        site_builder::run(args).await
    };

    deploy(
        site_dir.path().to_owned(),
        cluster.sites_config_path().to_owned(),
    )
    .await?;

    let site_id = *cluster.last_site_created().await?.id.object_id();
    assert!(cluster.site_redirects(site_id).await?.is_none());

    // Deploy again with redirects.
    let ws: WSResources = serde_json::from_reader(File::open(&ws_path)?)?;
    serde_json::to_writer_pretty(
        BufWriter::new(File::create(&ws_path)?),
        &WSResources {
            redirects: Some(make_redirects(&[("/go", "https://example.com", 303)])),
            object_id: ws.object_id,
            ..Default::default()
        },
    )?;

    deploy(
        site_dir.path().to_owned(),
        cluster.sites_config_path().to_owned(),
    )
    .await?;

    let r = cluster.site_redirects(site_id).await?;
    let r = r.expect("redirects should now exist on chain");
    assert_eq!(r.0.len(), 1);
    assert_eq!(r.0 .0["/go"].location, "https://example.com");
    assert_eq!(r.0 .0["/go"].status_code, 303);

    Ok(())
}

#[tokio::test]
#[ignore]
async fn publish_with_redirects_and_routes() -> anyhow::Result<()> {
    let cluster = TestSetup::start_local_test_cluster(None).await?;

    let site_dir = tempfile::tempdir()?;
    for i in 0..5 {
        let mut f = File::create(site_dir.path().join(format!("page_{i}.html")))?;
        writeln!(f, "<html><body>Page {i}</body></html>")?;
    }

    let routes = Routes(VecMap(BTreeMap::from([
        ("/alias-0".to_string(), "/page_0.html".to_string()),
        ("/alias-1".to_string(), "/page_1.html".to_string()),
    ])));
    let redirects = make_redirects(&[
        ("/old-0", "/page_2.html", 301),
        ("/old-1", "https://example.com", 302),
    ]);

    let ws_dir = tempfile::tempdir()?;
    let ws_path = write_ws_resources(
        ws_dir.path(),
        &WSResources {
            routes: Some(routes),
            redirects: Some(redirects),
            ..Default::default()
        },
    )?;

    let args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Publish {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(site_dir.path().to_owned())
                .with_ws_resources(Some(ws_path))
                .with_epoch_count_or_max(EpochCountOrMax::Max)
                .build()?,
            site_name: None,
        })
        .build()?;
    site_builder::run(args).await?;

    let site_id = *cluster.last_site_created().await?.id.object_id();

    let resources = cluster.site_resources(site_id).await?;
    assert_eq!(resources.len(), 5);

    let on_chain_routes = cluster.site_routes(site_id).await?;
    let on_chain_routes = on_chain_routes.expect("routes should exist on chain");
    assert_eq!(on_chain_routes.0.len(), 2);

    let on_chain_redirects = cluster.site_redirects(site_id).await?;
    let on_chain_redirects = on_chain_redirects.expect("redirects should exist on chain");
    assert_eq!(on_chain_redirects.0.len(), 2);

    Ok(())
}

#[tokio::test]
#[ignore]
async fn publish_with_many_redirects() -> anyhow::Result<()> {
    const N_REDIRECTS: usize = 200;

    let cluster = TestSetup::start_local_test_cluster(None).await?;

    let site_dir = tempfile::tempdir()?;
    for i in 0..3 {
        let mut f = File::create(site_dir.path().join(format!("page_{i}.html")))?;
        writeln!(f, "<html><body>Page {i}</body></html>")?;
    }

    let mut owned_entries: Vec<(String, String, u16)> = Vec::new();
    for i in 0..N_REDIRECTS {
        owned_entries.push((
            format!("/redirect/{i:04}/from/some/long/path/to/trigger/splitting"),
            format!("https://example.com/target/{i:04}"),
            if i % 2 == 0 { 301 } else { 302 },
        ));
    }
    let redirects = Redirects(VecMap(
        owned_entries
            .iter()
            .map(|(path, location, code)| {
                (
                    path.clone(),
                    Redirect {
                        location: location.clone(),
                        status_code: *code,
                    },
                )
            })
            .collect(),
    ));

    let ws_dir = tempfile::tempdir()?;
    let ws_path = write_ws_resources(
        ws_dir.path(),
        &WSResources {
            redirects: Some(redirects),
            ..Default::default()
        },
    )?;

    let args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Publish {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(site_dir.path().to_owned())
                .with_ws_resources(Some(ws_path))
                .with_epoch_count_or_max(EpochCountOrMax::Max)
                .build()?,
            site_name: None,
        })
        .with_gas_budget(100_000_000_000)
        .build()?;
    site_builder::run(args).await?;

    let site_id = *cluster.last_site_created().await?.id.object_id();

    let on_chain_redirects = cluster.site_redirects(site_id).await?;
    let on_chain_redirects = on_chain_redirects.expect("redirects should exist on chain");
    assert_eq!(on_chain_redirects.0.len(), N_REDIRECTS);

    Ok(())
}

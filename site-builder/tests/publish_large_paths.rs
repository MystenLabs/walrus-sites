// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Integration test for PTB byte-size batching.
//!
//! 100 HTML files with ~500-char nested paths plus 100 routes with ~500-char keys inflate PTB
//! payload well over `PTB_MAX_BYTES = 50,000`, forcing the site builder to split operations
//! across multiple PTBs.

use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{BufWriter, Write},
};

use site_builder::{
    args::{Commands, EpochCountOrMax},
    site_config::WSResources,
    types::{Routes, VecMap},
};

#[allow(dead_code)]
mod localnode;
use localnode::{
    args_builder::{ArgsBuilder, PublishOptionsBuilder},
    TestSetup,
};

/// Generates a long nested path of approximately `path_length` characters.
///
/// The `prefix` and `suffix` are used as the start and end of the path, with repeated
/// `/segment_NNNN` segments in between to reach the desired length.
pub fn generate_long_path(prefix: &str, suffix: &str, path_length: usize) -> String {
    let mut segments = prefix.to_string();
    let mut seg_idx = 0u32;
    while segments.len() + suffix.len() < path_length {
        segments.push_str(&format!("/segment_{seg_idx:04}"));
        seg_idx += 1;
    }
    segments.push_str(suffix);
    segments
}

/// Helper to create a test site with long nested directory paths.
///
/// Each file gets a deeply nested path of approximately `path_length` characters,
/// useful for testing PTB byte-size limits where long paths inflate transaction size.
///
/// Returns the temp directory and the list of generated relative paths (without leading `/`).
pub fn create_test_site_with_long_paths(
    n_files: usize,
    path_length: usize,
) -> anyhow::Result<(tempfile::TempDir, Vec<String>)> {
    use std::{
        collections::hash_map::DefaultHasher,
        hash::{Hash, Hasher},
    };

    let temp_dir = tempfile::tempdir()?;
    let directory = temp_dir.path();

    let mut hasher = DefaultHasher::new();
    directory.hash(&mut hasher);
    let unique_id = hasher.finish();

    let mut paths = Vec::with_capacity(n_files);
    for i in 0..n_files {
        // Build a long path by repeating nested segments.
        // Pattern: assets/components/features/section_{i:04}/seg_00/seg_01/.../page_{i:04}.html
        let prefix = format!("assets/components/features/section_{i:04}");
        let suffix = format!("/page_{i:04}.html");
        let segments = generate_long_path(&prefix, &suffix, path_length);

        // Create intermediate directories and the file
        let file_path = directory.join(&segments);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = File::create(&file_path)?;
        writeln!(file, "<html><body>")?;
        writeln!(file, "<h1>Test File {i}</h1>")?;
        writeln!(file, "<p>Path length: {}</p>", segments.len())?;
        writeln!(file, "<!-- Unique: {unique_id} / {i} -->")?;
        writeln!(file, "</body></html>")?;

        paths.push(segments);
    }

    Ok((temp_dir, paths))
}

#[tokio::test]
#[ignore]
async fn publish_site_with_large_resource_paths_and_routes() -> anyhow::Result<()> {
    const N_RESOURCES: usize = 100;
    const N_ROUTES: usize = 100;
    const PATH_LENGTH: usize = 500;

    let cluster = TestSetup::start_local_test_cluster(None).await?;

    // Create 100 files with ~500-char nested directory paths.
    let (site_temp_dir, resource_paths) =
        create_test_site_with_long_paths(N_RESOURCES, PATH_LENGTH)?;

    // Create 100 routes with ~500-char keys pointing to the existing resources.
    // Route keys use a different prefix so they don't collide with resource paths.
    let routes = Routes(VecMap(
        (0..N_ROUTES)
            .map(|i| {
                let route_key = format!(
                    "/{}",
                    generate_long_path(
                        &format!("routes/redirects/category_{i:04}"),
                        &format!("/target_{i:04}"),
                        PATH_LENGTH,
                    )
                );
                let target_resource = format!("/{}", &resource_paths[i]);
                (route_key, target_resource)
            })
            .collect::<BTreeMap<_, _>>(),
    ));

    // Write ws-resources.json with routes into a separate temp dir.
    let ws_resources = WSResources {
        routes: Some(routes),
        ..Default::default()
    };
    let ws_temp_dir = tempfile::tempdir()?;
    let ws_resources_path = ws_temp_dir.path().join("ws-resources.json");
    let file = File::create(&ws_resources_path)?;
    serde_json::to_writer_pretty(BufWriter::new(file), &ws_resources)?;

    let args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Publish {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(site_temp_dir.path().to_owned())
                .with_ws_resources(Some(ws_resources_path))
                .with_epoch_count_or_max(EpochCountOrMax::Max)
                .build()?,
            site_name: None,
        })
        .with_gas_budget(100_000_000_000)
        .build()?;

    site_builder::run(args).await.inspect_err(|e| {
        println!("error running site-builder: {e}");
    })?;

    let site = cluster.last_site_created().await?;
    let resources = cluster.site_resources(*site.id.object_id()).await?;
    assert_eq!(
        resources.len(),
        N_RESOURCES,
        "All {N_RESOURCES} resources should be created on-chain"
    );

    // Verify that multiple PTBs were used (byte-size guard triggered splitting).
    let txs = cluster
        .transaction_blocks_with_functions(&[("site", "new_resource"), ("site", "insert_route")])
        .await?;
    for (i, tx) in txs.iter().enumerate() {
        let raw_size = tx.raw_transaction.len();
        let n_commands = tx.transaction.as_ref().unwrap().data.move_calls().len();
        println!(
            "  PTB {i}: digest={}, raw tx size={raw_size} bytes, move calls={n_commands}",
            tx.digest
        );
    }
    let tx_count = txs.len();
    assert!(
        tx_count >= 2,
        "Expected >= 2 transaction blocks for new_resource, got {tx_count}. \
         The byte-size guard should split operations across multiple PTBs."
    );

    println!(
        "Successfully published {N_RESOURCES} resources + {N_ROUTES} routes \
         (~{PATH_LENGTH}-char paths) across {tx_count} transaction blocks"
    );

    Ok(())
}

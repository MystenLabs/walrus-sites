// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Tests for quilts dry-run functionality regression.
//!
//! These tests specifically target the bug that was fixed where dry-run mode
//! would consume the chunks iterator during cost estimation, causing resource
//! processing to fail after user confirmation with "Transaction effects not found".

#![cfg(feature = "quilts-experimental")]

use std::{
    fs::File,
    io::{Read, Write},
    num::NonZeroU32,
    path::PathBuf,
};

use site_builder::{
    args::{Commands, EpochCountOrMax},
    site_config::WSResources,
};

#[allow(dead_code)]
mod localnode;
use localnode::{
    args_builder::{ArgsBuilder, PublishOptionsBuilder},
    TestSetup,
};

#[allow(dead_code)]
mod helpers;

/// Extract cost value from a line containing FROST amount
fn extract_cost_from_frost_line(line: &str) -> Option<u128> {
    let frost_idx = line.rfind("FROST")?;
    let before_frost = line[..frost_idx].trim();
    let num_str = before_frost.split_whitespace().last()?;
    num_str.parse::<u128>().ok()
}

/// Helper function to parse estimated cost from captured output
fn parse_estimated_cost_from_output(output: &str) -> Option<u128> {
    // Look for "Estimated Storage Cost for this publish/update (Gas Cost Excluded): X FROST"
    output
        .lines()
        .find(|line| line.contains("Estimated Storage Cost") && line.contains("FROST"))
        .and_then(extract_cost_from_frost_line)
}

/// Test dry-run mode with both small (snake) and large sites.
/// This tests that the chunks iterator is not consumed during dry-run.
/// Tests are combined into one to avoid gag stdout redirect conflicts when running in parallel.
#[tokio::test]
async fn dry_run_both_sites_sync() -> anyhow::Result<()> {
    // Test small site (snake example)
    test_dry_run("snake", 4).await?;

    // Test large site (150 files)
    test_dry_run("large", 150).await?;

    Ok(())
}

/// Helper function to test dry-run execution.
async fn test_dry_run(site_type: &str, expected_file_count: usize) -> anyhow::Result<()> {
    let mut cluster = TestSetup::start_local_test_cluster().await?;

    // Get the wallet address for balance checking
    let wallet_address = cluster.wallet.inner.active_address()?;

    // Get the Walrus coin type from the admin client
    // The SuiContractClient should have the coin type information
    let walrus_sui_client = cluster.cluster_state.walrus_admin_client.inner.sui_client();
    let frost_coin_type = walrus_sui_client.read_client().wal_coin_type().to_string();

    // Get initial FROST balance
    let initial_balance = cluster
        .client
        .coin_read_api()
        .get_balance(wallet_address, Some(frost_coin_type.clone()))
        .await?;

    println!(
        "Initial FROST balance: {} FROST",
        initial_balance.total_balance
    );

    let temp_dir = tempfile::tempdir()?;
    let directory = temp_dir.path().to_path_buf();

    // Create test site based on type
    match site_type {
        "snake" => {
            // Copy snake example
            let snake_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join("examples")
                .join("snake");
            helpers::copy_dir(snake_dir.as_path(), directory.as_path())?;
        }
        "large" => {
            // Create a large site with many files
            create_large_test_site(directory.as_path(), expected_file_count)?;
        }
        _ => panic!("Unknown site type: {site_type}"),
    }

    // Reset object_id for new site creation
    let ws_resources_path = directory.join("ws-resources.json");
    let mut ws_resources: WSResources = if ws_resources_path.exists() {
        serde_json::from_reader(File::open(ws_resources_path.as_path())?)?
    } else {
        // Create default ws-resources.json for large site
        WSResources {
            headers: None,
            routes: None,
            metadata: None,
            site_name: Some(format!("Test {site_type} Site")),
            object_id: None,
            ignore: None,
        }
    };
    ws_resources.object_id = None; // Ensure this is treated as a new site
    serde_json::to_writer_pretty(File::create(ws_resources_path.as_path())?, &ws_resources)?;

    // Build args for dry-run publish
    let args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_gas_budget(50_000_000_000) // Same gas budget as publish_quilts.rs
        .with_command(Commands::PublishQuilts {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory.clone())
                .with_ws_resources(Some(ws_resources_path))
                .with_epoch_count_or_max(EpochCountOrMax::Epochs(NonZeroU32::new(1).unwrap()))
                .with_dry_run(true) // Enable dry-run mode
                .build()?,
            site_name: None,
        })
        .build()?;

    // This will use cfg(test) to auto-proceed past the confirmation prompt
    // The dry-run will print "Estimated Storage Cost: X FROST" to stdout,
    // then proceed with actual publish after confirmation

    // Capture stdout to extract the estimated cost
    // Wrap in a scope to ensure cleanup happens
    let (result, estimated_cost) = {
        let mut buf = gag::BufferRedirect::stdout().unwrap_or_else(|e| {
            panic!("Failed to redirect stdout (maybe already redirected?): {e}");
        });

        let result = site_builder::run(args).await;

        // Read captured output before dropping the buffer
        let mut output = String::new();
        buf.read_to_string(&mut output)
            .expect("Failed to read captured output");

        // Parse the estimated cost from captured output
        let estimated_cost = parse_estimated_cost_from_output(&output);

        // Drop buffer to restore stdout BEFORE printing
        drop(buf);

        // Print the captured output so it's visible in test output
        print!("{output}");

        (result, estimated_cost)
    };

    // Get final FROST balance after publish (if successful)
    let final_balance = if result.is_ok() {
        Some(
            cluster
                .client
                .coin_read_api()
                .get_balance(wallet_address, Some(frost_coin_type.clone()))
                .await?,
        )
    } else {
        None
    };

    // Check that we didn't hit the specific bugs we're testing for
    if let Err(e) = &result {
        let error_msg = format!("{e:?}");

        // Check for the iterator consumption bug (affects large sites)
        assert!(
            !error_msg.contains("Transaction effects not found"),
            "Command failed with iterator consumption bug: {error_msg}"
        );

        // Check for the object ID panic (affects small sites)
        assert!(
            !error_msg.contains("could not find the object ID for the created Walrus site"),
            "Command failed with object ID panic: {error_msg}"
        );
    }

    // Verify FROST cost if publish was successful
    if let Some(final_balance) = final_balance {
        let actual_cost = initial_balance.total_balance - final_balance.total_balance;

        println!("\n=== FROST Cost Verification ===");
        println!(
            "Initial FROST balance: {} FROST",
            initial_balance.total_balance
        );
        println!(
            "Final FROST balance:   {} FROST",
            final_balance.total_balance
        );
        println!("Actual FROST cost:     {actual_cost} FROST");
        if let Some(est) = estimated_cost {
            println!("Estimated FROST cost:  {est} FROST");
        }
        println!("================================\n");

        // Verify that FROST was actually spent
        assert!(
            actual_cost > 0,
            "Expected FROST to be spent, but balance did not decrease. \
             Initial: {}, Final: {}",
            initial_balance.total_balance,
            final_balance.total_balance
        );

        // Verify the actual cost matches the dry-run estimate
        if let Some(est) = estimated_cost {
            assert_eq!(
                actual_cost, est,
                "Actual FROST cost ({actual_cost}) does not match dry-run estimate ({est})",
            );
        } else {
            eprintln!("Warning: Could not parse estimated cost from output. Skipping exact match assertion.");
        }
    }

    // The test passes if we got here without hitting the specific bugs
    // Note: The actual publish may fail for other reasons (like network issues),
    // but as long as we don't hit the iterator consumption bug, the test passes
    Ok(())
}

/// Helper function to create a large test site with many files.
fn create_large_test_site(directory: &std::path::Path, file_count: usize) -> anyhow::Result<()> {
    // Create main index file
    let mut index_file = File::create(directory.join("index.html"))?;
    writeln!(index_file, "<!DOCTYPE html>")?;
    writeln!(
        index_file,
        "<html><head><title>Large Test Site</title></head>"
    )?;
    writeln!(
        index_file,
        "<body><h1>Large Test Site with {file_count} files</h1>"
    )?;
    writeln!(index_file, "<ul>")?;

    // Create many HTML files
    for i in 1..file_count {
        let file_name = format!("page_{i:03}.html");
        let file_path = directory.join(&file_name);

        let mut file = File::create(&file_path)?;
        writeln!(file, "<!DOCTYPE html>")?;
        writeln!(file, "<html><head><title>Page {i}</title></head>")?;
        writeln!(file, "<body>")?;
        writeln!(file, "<h1>This is page number {i}</h1>")?;
        writeln!(
            file,
            "<p>This is a test page with some content to make it non-trivial.</p>"
        )?;
        writeln!(
            file,
            "<p>Page generated for testing dry-run functionality with large sites.</p>"
        )?;
        writeln!(file, "<a href=\"index.html\">Back to main page</a>")?;
        writeln!(file, "</body></html>")?;

        // Add to index
        writeln!(
            index_file,
            "<li><a href=\"{file_name}\">{file_name}</a></li>"
        )?;
    }

    writeln!(index_file, "</ul></body></html>")?;

    // Create a CSS file
    let mut css_file = File::create(directory.join("style.css"))?;
    writeln!(
        css_file,
        "body {{ font-family: Arial, sans-serif; margin: 20px; }}"
    )?;
    writeln!(css_file, "h1 {{ color: #333; }}")?;
    writeln!(css_file, "a {{ color: #0066cc; text-decoration: none; }}")?;
    writeln!(css_file, "a:hover {{ text-decoration: underline; }}")?;

    println!(
        "Created large test site with {} files in {}",
        file_count,
        directory.display()
    );
    Ok(())
}

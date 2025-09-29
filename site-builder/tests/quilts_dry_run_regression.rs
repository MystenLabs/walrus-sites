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
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

use site_builder::site_config::WSResources;

#[allow(dead_code)]
mod localnode;
use localnode::TestSetup;

mod helpers;

/// Test dry-run mode with snake example (small site) - subprocess with simulated user input.
#[tokio::test]
async fn dry_run_snake_with_simulated_yes_input() -> anyhow::Result<()> {
    test_subprocess_dry_run("snake", true, 4).await // Snake has ~4 files
}

/// Test dry-run mode with snake example (small site) - subprocess without input (should timeout).
#[tokio::test]
async fn dry_run_snake_without_input() -> anyhow::Result<()> {
    test_subprocess_dry_run("snake", false, 4).await
}

/// Test dry-run mode with large site (150 files) - subprocess with simulated user input.
#[tokio::test]
async fn dry_run_large_site_with_simulated_yes_input() -> anyhow::Result<()> {
    test_subprocess_dry_run("large", true, 150).await
}

/// Test dry-run mode with large site (150 files) - subprocess without input (should timeout).
#[tokio::test]
async fn dry_run_large_site_without_input() -> anyhow::Result<()> {
    test_subprocess_dry_run("large", false, 150).await
}

/// Helper function to test dry-run subprocess execution.
async fn test_subprocess_dry_run(
    site_type: &str,
    provide_input: bool,
    expected_file_count: usize,
) -> anyhow::Result<()> {
    let cluster = TestSetup::start_local_test_cluster().await?;
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

    // Spawn the subprocess
    let mut child = Command::new("cargo")
        .args([
            "run",
            "--features",
            "quilts-experimental",
            "--",
            "--config",
            cluster.sites_config_path().to_str().unwrap(),
            "publish-quilts",
            "--epochs",
            "1",
            "--dry-run",
            directory.to_str().unwrap(), // directory is a positional argument
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // Provide input if requested
    if provide_input {
        if let Some(stdin) = child.stdin.as_mut() {
            stdin.write_all(b"y\n")?;
            stdin.flush()?;
        }
    }

    let output = child.wait_with_output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    println!("=== Test: {site_type} site, input: {provide_input} ===");
    println!("Exit status: {:?}", output.status);
    println!("Stdout: {stdout}");
    println!("Stderr: {stderr}");

    if provide_input {
        // When we provide input, the command should either succeed or fail gracefully
        // but NOT with the specific bugs we're testing for
        if !output.status.success() {
            // Check for the iterator consumption bug (affects large sites)
            assert!(
                !stderr.contains("Transaction effects not found"),
                "Command failed with iterator consumption bug: {stderr}"
            );

            // Check for the object ID panic (affects small sites)
            assert!(
                !stderr.contains("could not find the object ID for the created Walrus site"),
                "Command failed with object ID panic: {stderr}"
            );

            // General panic check
            assert!(!stderr.contains("panic"), "Command panicked: {stderr}");
        }
    } else {
        // When we don't provide input, it should fail due to terminal/IO issues
        // but still NOT with the specific bugs we're testing for
        assert!(
            !output.status.success(),
            "Expected command to fail without input"
        );

        // Should fail with terminal/IO error, not with our specific bugs
        assert!(
            !stderr.contains("Transaction effects not found"),
            "Command failed with iterator consumption bug even without user input: {stderr}"
        );

        // The process should reach the confirmation prompt before failing
        assert!(
            stderr.contains("Waiting for user confirmation")
                || stdout.contains("Waiting for user confirmation"),
            "Should reach confirmation prompt: stdout: {stdout} stderr: {stderr}"
        );
    }

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

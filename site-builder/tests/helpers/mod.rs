// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::HashSet,
    fs::{self, File},
    io::{self, Write},
    path::Path,
    time::SystemTime,
};

use site_builder::types::SuiResource;
use sui_types::base_types::SuiAddress;
use walrus_sdk::core::BlobId;

use crate::localnode::{TestBlob, TestSetup};

pub fn copy_dir(src: &Path, dst: &Path) -> io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;

        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Helper to create a simple test site with a few files.
/// Adds a unique identifier to prevent blob deduplication across different test runs.
pub fn create_test_site(directory: &std::path::Path, num_files: usize) -> anyhow::Result<()> {
    // Use directory path hash as a unique identifier to prevent blob deduplication across tests
    use std::{
        collections::hash_map::DefaultHasher,
        hash::{Hash, Hasher},
    };

    let mut hasher = DefaultHasher::new();
    directory.hash(&mut hasher);
    let unique_id = hasher.finish();

    for i in 0..num_files {
        let file_path = directory.join(format!("file_{i}.html"));
        let mut file = File::create(file_path)?;
        writeln!(file, "<html><body>")?;
        writeln!(file, "<h1>Test File {i}</h1>")?;
        writeln!(file, "<!-- Unique test ID: {unique_id} -->")?;
        writeln!(file, "</body></html>")?;
    }

    Ok(())
}

/// Get blobs owned by wallet filtered to only those matching the given resources.
/// This is useful when multiple tests run in sequence and you want to verify
/// only the blobs created for a specific site.
pub async fn get_blobs_for_resources(
    cluster: &TestSetup,
    wallet_address: SuiAddress,
    resources: &[SuiResource],
) -> anyhow::Result<Vec<TestBlob>> {
    let all_blobs = cluster.get_owned_blobs(wallet_address).await?;
    let resource_blob_ids: HashSet<BlobId> =
        resources.iter().map(|r| BlobId(r.blob_id.0)).collect();

    Ok(all_blobs
        .into_iter()
        .filter(|b| resource_blob_ids.contains(&b.blob_id))
        .collect())
}

/// Calculate the minimum expected end_epoch for a given expiry time.
/// This accounts for the current time, epoch duration, and rounds up to ensure
/// the blob will still be valid at the expiry time.
pub fn calculate_min_end_epoch_for_expiry(
    expiry_time: SystemTime,
    current_epoch: u32,
    epoch_duration_ms: u64,
) -> anyhow::Result<u32> {
    let now = SystemTime::now();
    let duration_until_expiry = expiry_time
        .duration_since(now)
        .map_err(|e| anyhow::anyhow!("Expiry time is in the past: {}", e))?;
    let ms_until_expiry = duration_until_expiry.as_millis() as u64;

    // Ceiling division to ensure we have enough epochs
    let epochs_until_expiry = ms_until_expiry.div_ceil(epoch_duration_ms);

    Ok(current_epoch + epochs_until_expiry as u32)
}

/// Helper to create a test site with large files containing random text data.
/// This is useful for testing quilt size limits and large file handling.
///
/// # Arguments
/// * `directory` - The directory where files will be created
/// * `n_files` - Number of files to create
/// * `size_per_file_bytes` - Approximate size of each file in bytes
///
/// # Best Practice
/// The caller should create and manage the temp directory using `tempfile::tempdir()`.
/// This follows the pattern of `create_test_site` and provides better control over
/// the directory lifecycle.
///
/// # Example
/// ```no_run
/// let temp_dir = tempfile::tempdir()?;
/// create_large_test_site(temp_dir.path(), 10, 1024 * 1024)?; // 10 files of ~1MB each
/// ```
pub fn create_large_test_site(
    directory: &Path,
    n_files: usize,
    size_per_file_bytes: usize,
) -> anyhow::Result<()> {
    use rand::{
        distributions::{Alphanumeric, DistString},
        SeedableRng,
    };

    // Use a seeded RNG for reproducibility in tests
    let mut rng = rand::rngs::StdRng::from_entropy();

    for i in 0..n_files {
        let file_path = directory.join(format!("large_file_{i}.html"));
        let mut file = File::create(file_path)?;

        // Prepare HTML header and footer as strings to measure exact size
        let html_header = format!(
            "<!DOCTYPE html>\n<html><head><title>Large Test File {i}</title></head>\n<body>\n<h1>Large Test File {i}</h1>\n<p>"
        );
        let html_footer = "</p>\n</body></html>\n";

        // Calculate exact HTML overhead
        let html_overhead = html_header.len() + html_footer.len();

        // Calculate how much random text we need to generate
        let text_size = if size_per_file_bytes > html_overhead {
            size_per_file_bytes - html_overhead
        } else {
            size_per_file_bytes
        };

        // Write HTML header
        write!(file, "{html_header}")?;

        // Generate random alphanumeric text in chunks to avoid memory issues
        const CHUNK_SIZE: usize = 8192; // 8KB chunks
        let full_chunks = text_size / CHUNK_SIZE;
        let remainder = text_size % CHUNK_SIZE;

        // Write full chunks
        for _ in 0..full_chunks {
            let chunk = Alphanumeric.sample_string(&mut rng, CHUNK_SIZE);
            write!(file, "{chunk}")?;
        }

        // Write remainder
        if remainder > 0 {
            let chunk = Alphanumeric.sample_string(&mut rng, remainder);
            write!(file, "{chunk}")?;
        }

        // Write HTML footer
        write!(file, "{html_footer}")?;
    }

    Ok(())
}

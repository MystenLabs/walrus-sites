// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashSet, fs, io, path::Path, time::SystemTime};

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

// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Quilt storage manager for Walrus Sites.

use std::num::NonZeroU16;

use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use bytesize::ByteSize;
use itertools::Itertools;

use super::resource::{Resource, ResourceData, MAX_IDENTIFIER_SIZE};
use crate::{
    args::EpochArg,
    display,
    walrus::{
        command::{QuiltBlobInput, StoreQuiltInput},
        types::BlobId,
        Walrus,
    },
};

/// Information for dry-run mode.
///
/// When passed to `store_quilts`, enables dry-run mode which shows cost estimates
/// and asks for user confirmation before proceeding.

/// Manages quilt storage operations for Walrus Sites.
pub struct QuiltsManager {
    /// The controller for the Walrus CLI.
    pub walrus: Walrus,
    /// The number of shards of the Walrus system.
    pub n_shards: NonZeroU16,
}

impl QuiltsManager {
    /// Creates a new quilts manager.
    pub async fn new(walrus: Walrus) -> Result<Self> {
        let n_shards = walrus.n_shards().await?;
        Ok(QuiltsManager { walrus, n_shards })
    }

    /// Store changed resources into quilts.
    ///
    /// Returns the stored resources as a vector.
    pub async fn store_quilts(
        &mut self,
        changed_resources: Vec<ResourceData>,
        epochs: EpochArg,
        max_quilt_size: ByteSize,
    ) -> Result<Vec<Resource>> {
        let chunks = self.quilts_chunkify(changed_resources, max_quilt_size)?;

        let mut resources = vec![];
        tracing::debug!("Processing chunks for quilt storage");

        for chunk in chunks {
            let chunk_resources = self
                .store_resource_chunk_into_quilt(chunk, epochs.clone())
                .await?;
            resources.extend(chunk_resources);
        }
        Ok(resources)
    }

    async fn store_resource_chunk_into_quilt(
        &mut self,
        res_data_and_quilt_files: impl IntoIterator<Item = (ResourceData, QuiltBlobInput)>,
        epochs: EpochArg,
    ) -> Result<Vec<Resource>> {
        let (resource_data, quilt_blob_inputs): (Vec<_>, Vec<_>) =
            res_data_and_quilt_files.into_iter().unzip();
        let mut store_resp = self
            .walrus
            .store_quilt(
                StoreQuiltInput::Blobs(quilt_blob_inputs.clone()),
                epochs,
                false,
                true,
            )
            .await
            .context(format!(
                "error while storing the quilt for resources: {}",
                quilt_blob_inputs
                    .iter()
                    .map(|f| f.path.display())
                    .join(", ")
            ))?;

        let blob_id = *store_resp.blob_store_result.blob_id();
        let quilt_patches = &mut store_resp.stored_quilt_blobs;

        resource_data
            .into_iter()
            .map(|res_data| {
                let patch_identifier = res_data.resource_path.as_str();
                // Walrus store does not maintain the order the files were passed
                let Some(pos) = quilt_patches
                    .iter()
                    .position(|p| p.identifier == patch_identifier)
                else {
                    bail!(
                        "Resource {} exists but doesn't have a matching quilt-patch",
                        res_data.resource_path
                    );
                };
                let patch = quilt_patches.swap_remove(pos);
                let bytes = URL_SAFE_NO_PAD.decode(patch.quilt_patch_id.as_str())?;

                // Must have at least BlobId.LENGTH + 1 bytes (quilt_id + version).
                if bytes.len() != Walrus::QUILT_PATCH_ID_SIZE {
                    bail!(
                        "Expected {} bytes when decoding quilt-patch-id version 1.",
                        Walrus::QUILT_PATCH_ID_SIZE
                    );
                }

                // Extract patch_id (bytes after the blob_id).
                let patch_bytes: [u8; Walrus::QUILT_PATCH_SIZE] = bytes
                    [BlobId::LENGTH..Walrus::QUILT_PATCH_ID_SIZE]
                    .try_into()
                    .unwrap();
                let version = patch_bytes[0];
                if version != Walrus::QUILT_PATCH_VERSION_1 {
                    bail!("Quilt patch version {version} is not implemented");
                }

                let patch_hex = format!("0x{}", hex::encode(patch_bytes));
                Ok(res_data.into_resource(blob_id, Some(patch_hex)))
            })
            .collect::<Result<Vec<_>>>()
    }

    pub async fn dry_run_resource_chunk(
        &mut self,
        quilt_blob_inputs: Vec<QuiltBlobInput>,
        epochs: EpochArg,
    ) -> Result<u64> {
        let store_resp = self
            .walrus
            .dry_run_store_quilt(
                StoreQuiltInput::Blobs(quilt_blob_inputs.clone()),
                epochs,
                false,
                true,
            )
            .await
            .context(format!(
                "error while computing the blob id for resources in path: {}",
                quilt_blob_inputs
                    .iter()
                    .map(|f| f.path.display())
                    .join(", ")
            ))?;
        Ok(store_resp.quilt_blob_output.storage_cost)
    }

    /// Get the chunks for storing resources into quilts.
    pub fn quilts_chunkify(
        &self,
        resources: Vec<ResourceData>,
        max_quilt_size: ByteSize,
    ) -> Result<Vec<Vec<(ResourceData, QuiltBlobInput)>>> {
        let max_quilt_size = max_quilt_size.as_u64() as usize;
        let max_available_columns = Walrus::max_slots_in_quilt(self.n_shards) as usize;
        let max_theoretical_quilt_size =
            Walrus::max_slot_size(self.n_shards) * max_available_columns;

        // Cap the effective_quilt_size to the min between the theoretical and the passed
        let effective_quilt_size = if max_theoretical_quilt_size < max_quilt_size {
            display::warning(format!(
                "Configured max quilt size ({}) exceeds theoretical maximum ({}). Using {} instead.",
                ByteSize(max_quilt_size as u64),
                ByteSize(max_theoretical_quilt_size as u64),
                ByteSize(max_theoretical_quilt_size as u64)
            ));
            max_theoretical_quilt_size
        } else {
            max_quilt_size
        };
        let mut available_columns = max_available_columns;
        // Calculate capacity per column (slot) in bytes
        let column_capacity = effective_quilt_size / available_columns;
        // Per-file overhead constant
        const FIXED_OVERHEAD: usize = 8; // BLOB_IDENTIFIER_SIZE_BYTES_LENGTH (2) + BLOB_HEADER_SIZE (6)

        let mut chunks = vec![];
        let mut current_chunk = vec![];

        for res_data in resources.into_iter() {
            let quilt_input: QuiltBlobInput = (&res_data).into();
            // Calculate total size including overhead
            let file_size_with_overhead =
                res_data.unencoded_size() + MAX_IDENTIFIER_SIZE + FIXED_OVERHEAD;

            // Abort if the file cannot fit even in the theoretical maximum
            if file_size_with_overhead > max_theoretical_quilt_size {
                anyhow::bail!(
                    "File '{}' with size {} exceeds Walrus theoretical maximum of {} for single file storage. \
                    This file cannot be stored in Walrus with the current shard configuration.",
                    res_data.full_path().display(),
                    ByteSize(file_size_with_overhead as u64),
                    ByteSize(max_theoretical_quilt_size as u64)
                );
            }

            // If file exceeds effective_quilt_size but is below theoretical limit,
            // place it alone in its own chunk and continue with the current chunk
            if file_size_with_overhead > effective_quilt_size {
                // Place large file in its own chunk (don't save current_chunk yet)
                chunks.push(vec![(res_data, quilt_input)]);
                // Continue filling the current chunk with remaining capacity
                continue;
            }

            // Calculate how many columns this file needs
            let columns_needed = file_size_with_overhead.div_ceil(column_capacity);

            if available_columns >= columns_needed {
                // File fits in current chunk
                current_chunk.push((res_data, quilt_input));
                available_columns -= columns_needed;
            } else {
                // File doesn't fit, start a new chunk
                if !current_chunk.is_empty() {
                    chunks.push(current_chunk);
                }
                current_chunk = vec![(res_data, quilt_input)];
                // Reset available columns for new chunk
                available_columns = max_available_columns - columns_needed;
            }
        }

        // Push the last chunk if it's not empty
        if !current_chunk.is_empty() {
            chunks.push(current_chunk);
        }

        Ok(chunks)
    }

    /// Test-friendly version of quilts_chunkify that accepts n_shards as a parameter.
    #[cfg(test)]
    pub fn quilts_chunkify_with_n_shards(
        resources: Vec<ResourceData>,
        max_quilt_size: ByteSize,
        n_shards: NonZeroU16,
    ) -> Result<Vec<Vec<(ResourceData, QuiltBlobInput)>>> {
        let max_quilt_size = max_quilt_size.as_u64() as usize;
        let max_available_columns = Walrus::max_slots_in_quilt(n_shards) as usize;
        let max_theoretical_quilt_size = Walrus::max_slot_size(n_shards) * max_available_columns;

        // Cap the effective_quilt_size to the min between the theoretical and the passed
        let effective_quilt_size = if max_theoretical_quilt_size < max_quilt_size {
            display::warning(format!(
                "Configured max quilt size ({}) exceeds theoretical maximum ({}). Using {} instead.",
                ByteSize(max_quilt_size as u64),
                ByteSize(max_theoretical_quilt_size as u64),
                ByteSize(max_theoretical_quilt_size as u64)
            ));
            max_theoretical_quilt_size
        } else {
            max_quilt_size
        };
        let mut available_columns = max_available_columns;
        // Calculate capacity per column (slot) in bytes
        let column_capacity = effective_quilt_size / available_columns;
        // Per-file overhead constant
        const FIXED_OVERHEAD: usize = 8; // BLOB_IDENTIFIER_SIZE_BYTES_LENGTH (2) + BLOB_HEADER_SIZE (6)

        let mut chunks = vec![];
        let mut current_chunk = vec![];

        for res_data in resources.into_iter() {
            let quilt_input: QuiltBlobInput = (&res_data).into();
            // Calculate total size including overhead
            let file_size_with_overhead =
                res_data.unencoded_size() + MAX_IDENTIFIER_SIZE + FIXED_OVERHEAD;

            // Abort if the file cannot fit even in the theoretical maximum
            if file_size_with_overhead > max_theoretical_quilt_size {
                anyhow::bail!(
                    "File '{}' with size {} exceeds Walrus theoretical maximum of {} for single file storage. \
                    This file cannot be stored in Walrus with the current shard configuration.",
                    res_data.full_path().display(),
                    ByteSize(file_size_with_overhead as u64),
                    ByteSize(max_theoretical_quilt_size as u64)
                );
            }

            // If file exceeds effective_quilt_size but is below theoretical limit,
            // place it alone in its own chunk and continue with the current chunk
            if file_size_with_overhead > effective_quilt_size {
                // Place large file in its own chunk (don't save current_chunk yet)
                chunks.push(vec![(res_data, quilt_input)]);
                // Continue filling the current chunk with remaining capacity
                continue;
            }

            // Calculate how many columns this file needs
            let columns_needed = file_size_with_overhead.div_ceil(column_capacity);

            if available_columns >= columns_needed {
                // File fits in current chunk
                current_chunk.push((res_data, quilt_input));
                available_columns -= columns_needed;
            } else {
                // File doesn't fit, start a new chunk
                if !current_chunk.is_empty() {
                    chunks.push(current_chunk);
                }
                current_chunk = vec![(res_data, quilt_input)];
                // Reset available columns for new chunk
                available_columns = max_available_columns - columns_needed;
            }
        }

        // Push the last chunk if it's not empty
        if !current_chunk.is_empty() {
            chunks.push(current_chunk);
        }

        Ok(chunks)
    }
}

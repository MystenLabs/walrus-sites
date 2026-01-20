// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Estimation utilities for Walrus storage and Sui gas costs.
//!
//! This module provides functions for estimating costs without actually executing
//! operations, used in dry-run mode

use anyhow::Result;
use bcs;
use itertools::Itertools;
use serde::Serialize;
use sui_sdk::rpc_types::SuiTransactionBlockEffectsAPI;
use sui_types::{base_types::ObjectID, id::UID, transaction::CallArg};
use walrus_sdk::sui::utils::price_for_encoded_length;

use crate::{
    args::EpochArg,
    display,
    site::{
        builder::{SitePtbBuilderResultExt, PTB_MAX_MOVE_CALLS},
        manager::{BlobExtensions, SiteManager},
        quilts::QuiltsManager,
        resource::ResourceData,
    },
};

/// Conversion factor from MIST to SUI
const MIST_PER_SUI: f64 = 1_000_000_000.0;

/// A minimal Site struct for gas estimation purposes.
#[derive(Serialize)]
struct SiteForEstimation {
    id: UID,
    name: String,
    link: Option<String>,
    image_url: Option<String>,
    description: Option<String>,
    project_url: Option<String>,
    creator: Option<String>,
}

impl SiteForEstimation {
    /// Creates a minimal Site struct for gas estimation purposes.
    fn new_for_estimation() -> Self {
        Self {
            id: UID::new(ObjectID::ZERO),
            name: "Estimation".to_string(),
            link: None,
            image_url: None,
            description: None,
            project_url: None,
            creator: None,
        }
    }

    /// Serialize to BCS bytes for use as a Pure CallArg.
    fn to_bcs_bytes(&self) -> Vec<u8> {
        bcs::to_bytes(self).expect("SiteForEstimation serialization should not fail")
    }
}

/// Returns a `CallArg` using a fake BCS-serialized Site for gas estimation.
fn fake_site_call_arg() -> CallArg {
    CallArg::Pure(SiteForEstimation::new_for_estimation().to_bcs_bytes())
}

/// Handles all estimation operations for dry-run mode.
pub struct Estimator;

impl Estimator {
    /// Creates a new estimator.
    pub fn new() -> Self {
        Self
    }

    /// Estimates blob extension costs.
    pub fn estimate_blob_extensions(
        &self,
        blob_extensions: &BlobExtensions,
    ) -> Option<(usize, u64)> {
        match blob_extensions {
            BlobExtensions::Noop => None,
            BlobExtensions::Extend {
                blobs,
                new_end_epoch,
                storage_price,
            } => {
                let count = blobs.len();
                let total_cost: u64 = blobs
                    .iter()
                    .map(|(sui_blob, _)| {
                        let epochs_extended = *new_end_epoch - sui_blob.storage.end_epoch;
                        price_for_encoded_length(
                            sui_blob.storage.storage_size,
                            *storage_price,
                            epochs_extended,
                        )
                    })
                    .sum();
                Some((count, total_cost))
            }
        }
    }

    /// Shows Walrus storage estimates
    pub async fn show_walrus_estimates(
        &self,
        quilts_manager: &mut QuiltsManager,
        changed_resources: Vec<ResourceData>,
        epochs: EpochArg,
        max_quilt_size: bytesize::ByteSize,
        blob_extensions: &BlobExtensions,
    ) -> Result<()> {
        // Get chunks for estimation
        let chunks = quilts_manager.quilts_chunkify(changed_resources, max_quilt_size)?;

        // Calculate and display Walrus storage costs
        let mut total_storage_cost = 0;
        for chunk in &chunks {
            let quilt_file_inputs = chunk.iter().map(|(_, f)| f.clone()).collect_vec();
            let wal_storage_cost = quilts_manager
                .dry_run_resource_chunk(quilt_file_inputs, epochs.clone())
                .await?;
            total_storage_cost += wal_storage_cost;
        }

        display::header("Estimated Walrus Storage Cost for this publish/update:");
        display::info(format!("{total_storage_cost} FROST"));

        // Calculate and display blob extension estimates
        let extension_estimate = self.estimate_blob_extensions(blob_extensions);
        if let Some((blob_count, wal_cost)) = extension_estimate {
            display::action(format!(
                "Blob extensions: {wal_cost} FROST ({blob_count} blob{})",
                if blob_count == 1 { "" } else { "s" }
            ));
        }

        // Test mode handling
        #[cfg(feature = "_testing-dry-run")]
        {
            display::info("Test mode: automatically proceeding with estimates");
        }

        Ok(())
    }

    /// Shows Sui gas estimates for given site data.
    pub async fn show_sui_gas_estimates(
        &self,
        site_manager: &mut SiteManager,
        updates: &crate::site::SiteDataDiff<'_>,
        _blob_extensions: BlobExtensions,
        walrus_pkg: ObjectID,
    ) -> Result<u64> {
        tracing::debug!(
            address=?site_manager.active_address()?,
            "estimating Sui gas for site updates",
        );

        // Build the initial PTB
        let (initial_ptb, mut resources_iter, mut routes_iter) =
            site_manager.build_initial_ptb(updates, walrus_pkg).await?;

        let gas_ref = site_manager.gas_coin_ref().await?;

        // Dry run initial PTB
        let initial_response = site_manager
            .dry_run_ptb(initial_ptb.clone(), gas_ref)
            .await?;
        let initial_gas = initial_response.effects.gas_cost_summary().net_gas_usage() as u64;
        display::header("Sui gas estimates");
        display::info(format!(
            "Initial PTB gas cost: {} MIST ({:.3} SUI) ({} commands)",
            initial_gas,
            initial_gas as f64 / MIST_PER_SUI,
            initial_ptb.commands.len()
        ));

        // Check if we'll need additional PTBs by peeking at the iterators
        let has_remaining_resources =
            resources_iter.peek().is_some() || routes_iter.peek().is_some();

        if !has_remaining_resources {
            // No additional PTBs needed - just return the initial PTB gas cost
            display::header("Total estimated Sui gas cost");
            display::info(format!(
                "{} MIST ({:.2} SUI)",
                initial_gas,
                initial_gas as f64 / MIST_PER_SUI
            ));
            return Ok(initial_gas);
        }

        // Build remaining PTBs
        let fake_call_arg = fake_site_call_arg();
        let mut remaining_ptbs = Vec::new();

        while resources_iter.peek().is_some() || routes_iter.peek().is_some() {
            let ptb = site_manager.create_site_ptb::<{ PTB_MAX_MOVE_CALLS }>(walrus_pkg);
            let mut ptb = ptb.with_call_arg(&fake_call_arg)?;

            ptb.add_resource_operations(&mut resources_iter)
                .ok_if_limit_reached()?;

            // Add routes only if all resources have been added.
            if resources_iter.peek().is_none() {
                ptb.add_route_operations(&mut routes_iter)
                    .ok_if_limit_reached()?;
            }

            remaining_ptbs.push(ptb.finish());
        }

        // Dry run remaining PTBs
        let mut total_gas = initial_gas;
        if remaining_ptbs.is_empty() {
            display::info("Single transaction required for all updates");
        } else {
            display::info(format!(
                "Multiple transactions required: {} additional resource PTBs",
                remaining_ptbs.len()
            ));
        }

        for (i, ptb) in remaining_ptbs.iter().enumerate() {
            let gas_ref = site_manager.gas_coin_ref().await?;
            let response = site_manager.dry_run_ptb(ptb.clone(), gas_ref).await?;

            // If dev_inspect failed, estimate gas based on command count
            let gas_cost = if response.error.is_some() {
                // Use heuristic: scale based on command count compared to initial PTB
                let command_ratio = ptb.commands.len() as f64 / initial_ptb.commands.len() as f64;
                (initial_gas as f64 * command_ratio * 0.8) as u64 // Resource PTBs are typically simpler
            } else {
                response.effects.gas_cost_summary().net_gas_usage() as u64
            };

            total_gas += gas_cost;

            // Debug: show if there were any errors in dev_inspect
            if let Some(ref error) = response.error {
                display::error(format!(
                    "Resource PTB {}/{} had dev_inspect error: {}",
                    i + 1,
                    remaining_ptbs.len(),
                    error
                ));
                tracing::debug!("Estimated cost based on {} commands", ptb.commands.len());
            }

            display::info(format!(
                "Resource PTB {}/{}: {} MIST ({:.3} SUI) ({} commands)",
                i + 1,
                remaining_ptbs.len(),
                gas_cost,
                gas_cost as f64 / MIST_PER_SUI,
                ptb.commands.len()
            ));
        }

        display::header("Total estimated Sui gas cost");
        display::info(format!(
            "{} MIST ({:.3} SUI)",
            total_gas,
            total_gas as f64 / MIST_PER_SUI
        ));

        Ok(total_gas)
    }
}

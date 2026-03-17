// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! MVR (Move Registry) integration for package address resolution.

use anyhow::{Context, Result};
use serde::Deserialize;
use sui_types::base_types::ObjectID;

#[derive(Deserialize)]
struct MvrResponse {
    package_address: String,
}

/// Resolves the `@walrus/sites` package address from the MVR API.
///
/// `network` should be `"testnet"` or `"mainnet"` (derived from the config context name).
pub async fn resolve_walrus_sites_package(network: &str) -> Result<ObjectID> {
    let url = format!("https://{network}.mvr.mystenlabs.com/v1/names/@walrus/sites");
    tracing::info!(%url, "resolving walrus-sites package via MVR");

    let response: MvrResponse = reqwest::get(&url)
        .await
        .context("failed to reach MVR API")?
        .error_for_status()
        .context("MVR API returned an error status")?
        .json()
        .await
        .context("failed to parse MVR response")?;

    let package_id = response
        .package_address
        .parse::<ObjectID>()
        .context("invalid package_address in MVR response")?;

    tracing::info!(%package_id, "resolved walrus-sites package from MVR");
    Ok(package_id)
}

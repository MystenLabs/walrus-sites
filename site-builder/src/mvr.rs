// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! MVR (Move Registry) integration for package address resolution.

use std::time::Duration;

use anyhow::{Context, Result};
use serde::Deserialize;
use sui_types::base_types::ObjectID;
use walrus_sdk::core_utils::backoff::ExponentialBackoffConfig;

/// Timeout for a single MVR HTTP request.
const MVR_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Minimal subset of the MVR `/v1/names/` response — we only need `package_address`.
#[derive(Deserialize)]
struct MvrResponse {
    package_address: String,
}

/// Resolves the `@walrus/sites` package address from the MVR API.
///
/// `network` should be `"testnet"` or `"mainnet"` (derived from the config context name).
/// Retries with exponential backoff on transient network failures.
pub async fn resolve_walrus_sites_package(network: &str) -> Result<ObjectID> {
    let url = format!("https://{network}.mvr.mystenlabs.com/v1/names/@walrus/sites");
    tracing::info!(%url, "resolving walrus-sites package via MVR");

    let client = reqwest::Client::builder()
        .timeout(MVR_REQUEST_TIMEOUT)
        .build()
        .context("failed to build HTTP client for MVR")?;

    let backoff_config = ExponentialBackoffConfig::default();
    let mut backoff = backoff_config.get_strategy(rand::random());

    loop {
        match client.get(&url).send().await {
            Ok(resp) => {
                let response: MvrResponse = resp
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
                return Ok(package_id);
            }
            Err(e) => {
                if let Some(delay) = backoff.next() {
                    tracing::warn!(error = %e, ?delay, "MVR request failed, retrying");
                    tokio::time::sleep(delay).await;
                } else {
                    return Err(e).context("failed to reach MVR API after retries");
                }
            }
        }
    }
}

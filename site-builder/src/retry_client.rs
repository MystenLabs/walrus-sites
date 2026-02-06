// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Infrastructure for making RPC calls.
use anyhow::Context;
use sui_types::TypeTag;
use tracing::Level;
pub use walrus_sdk::sui::client::retry_client::RetriableSuiClient;
use walrus_sdk::{
    core_utils::backoff::ExponentialBackoffConfig,
    sui::client::retry_client::retriable_sui_client::LazySuiClientBuilder,
    ObjectID,
};

use crate::types::{Staking, StakingInnerV1, StakingObjectForDeserialization};

/// Creates a new [`RetriableSuiClient`] with the given RPC URL and backoff configuration.
pub fn new_retriable_sui_client(
    rpc_url: &str,
    backoff_config: ExponentialBackoffConfig,
) -> anyhow::Result<RetriableSuiClient> {
    RetriableSuiClient::new(
        vec![LazySuiClientBuilder::new(rpc_url, None)],
        backoff_config,
    )
}

/// Fetches the staking object by its ID.
///
/// Returns a [`Staking`] that includes version, package IDs, and staking parameters.
#[tracing::instrument(level = Level::DEBUG, skip_all)]
pub async fn get_staking_object(
    retriable_sui_client: &RetriableSuiClient,
    staking_object_id: ObjectID,
) -> anyhow::Result<Staking> {
    let StakingObjectForDeserialization {
        id,
        version,
        package_id,
        new_package_id,
    } = retriable_sui_client
        .get_sui_object(staking_object_id)
        .await
        .context("Failed to fetch staking object data")?;

    let inner = retriable_sui_client
        .get_dynamic_field::<u64, StakingInnerV1>(staking_object_id, TypeTag::U64, version)
        .await
        .context("Failed to fetch inner staking data")?;

    Ok(Staking {
        id,
        version,
        package_id,
        new_package_id,
        inner,
    })
}

// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Configuration types for interfacing with Walrus.

use std::{path::Path, time::Duration};

use anyhow::Context;
use serde::Deserialize;
use serde_with::{serde_as, DurationSeconds};
use sui_types::base_types::ObjectID;
use walrus_sdk::sui::client::{
    contract_config::ContractConfig,
    retry_client::RetriableSuiClient,
    ReadClient,
    SuiReadClient,
};

const fn default_cache_ttl() -> Duration {
    Duration::from_secs(10)
}

#[serde_as]
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
/// Configuration for the contract packages and shared objects.
pub struct WalrusContractConfig {
    /// Object ID of the Walrus system object.
    pub system_object: ObjectID,
    /// Object ID of the Walrus staking object.
    pub staking_object: ObjectID,
    /// Object ID of the credits object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credits_object: Option<ObjectID>,
    /// Object ID of the walrus subsidies object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub walrus_subsidies_object: Option<ObjectID>,
    /// The TTL for cached system and staking objects.
    #[serde(default = "default_cache_ttl", rename = "cache_ttl_secs")]
    #[serde_as(as = "DurationSeconds")]
    pub cache_ttl: Duration,
}

impl WalrusContractConfig {
    /// Load configuration from a YAML file.
    pub fn from_file(path: &Path) -> anyhow::Result<Self> {
        let contents =
            std::fs::read_to_string(path).context("Failed to read walrus config file")?;
        serde_yaml::from_str(&contents).context("Failed to parse walrus config file")
    }

    /// Creates a walrus-sdk `ContractConfig` from this config.
    pub fn to_sdk_contract_config(&self) -> ContractConfig {
        ContractConfig {
            system_object: self.system_object,
            staking_object: self.staking_object,
            credits_object: self.credits_object,
            walrus_subsidies_object: self.walrus_subsidies_object,
            cache_ttl: self.cache_ttl,
        }
    }

    // TODO(SEW-734): Migrate more functionality to use the walrus-sdk instead of duplicating
    // code from walrus-sui. Currently only `wal_coin_type` uses the SDK.
    /// Retrieves the WAL coin type string (e.g., `0x...::wal::WAL`) using the walrus SDK.
    ///
    /// This creates a `SuiReadClient` from the walrus SDK which fetches the WAL type
    /// from the walrus package's `StakedWal` struct definition on-chain.
    pub async fn wal_coin_type(
        &self,
        retriable_client: RetriableSuiClient,
    ) -> anyhow::Result<String> {
        let contract_config = self.to_sdk_contract_config();
        let sui_read_client = SuiReadClient::new(retriable_client, &contract_config).await?;
        Ok(sui_read_client.wal_coin_type().to_owned())
    }

    /// Retrieves the storage price per unit size (1 MiB) per epoch in FROST.
    pub async fn storage_price_per_unit_size(
        &self,
        retriable_client: RetriableSuiClient,
    ) -> anyhow::Result<u64> {
        let contract_config = self.to_sdk_contract_config();
        let sui_read_client = SuiReadClient::new(retriable_client, &contract_config).await?;
        Ok(sui_read_client.storage_price_per_unit_size().await?)
    }
}

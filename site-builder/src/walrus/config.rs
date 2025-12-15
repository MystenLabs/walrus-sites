// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Configuration types for interfacing with Walrus.

use std::{path::Path, time::Duration};

use anyhow::Context;
use serde::Deserialize;
use serde_with::{serde_as, DurationSeconds};
use sui_types::base_types::ObjectID;

use crate::args::default;

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
    #[serde(default = "default::cache_ttl", rename = "cache_ttl_secs")]
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
}

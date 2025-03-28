// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Configuration for the site builder.

use std::{collections::HashMap, path::Path};

use anyhow::{anyhow, Result};
use serde::Deserialize;
use sui_sdk::wallet_context::WalletContext;
use sui_types::base_types::ObjectID;

pub(crate) use crate::{args::GeneralArgs, walrus::Walrus};

/// Configuration for the site builder, complete with separate context for networks.
#[derive(Deserialize, Debug, Clone)]
pub(crate) struct MultiConfig {
    pub contexts: HashMap<String, Config>,
    pub default_context: String,
}

pub(crate) type ConfigWithContext = Config<String>;

/// The configuration for the site builder.
#[derive(Deserialize, Debug, Clone)]
pub(crate) struct Config<C = ()> {
    #[serde(default = "default_portal")]
    pub portal: String,
    pub package: ObjectID,
    #[serde(skip_deserializing)]
    pub context: C,
    #[serde(default)]
    pub general: GeneralArgs,
}

pub(crate) fn default_portal() -> String {
    "wal.app".to_owned()
}

impl<C> Config<C> {
    /// Merges the other [`GeneralArgs`] (taken from the CLI) with the `general` in the struct.
    ///
    /// The values in `other_general` take precedence.
    pub fn merge(&mut self, other_general: &GeneralArgs) {
        self.general.merge(other_general);
    }

    pub fn walrus_binary(&self) -> String {
        self.general
            .walrus_binary
            .as_ref()
            .expect("serde default => binary exists")
            .to_owned()
    }

    pub fn gas_budget(&self) -> u64 {
        self.general
            .gas_budget
            .expect("serde default => gas budget exists")
    }

    /// Returns a [`WalletContext`] from the configuration.
    pub fn load_wallet(&self) -> Result<WalletContext> {
        self.general.load_wallet()
    }

    /// Adds the context to the configuration.
    pub fn with_context(self, context: String) -> Config<String> {
        let Config {
            portal,
            package,
            general,
            ..
        } = self;
        Config {
            portal,
            package,
            general,
            context,
        }
    }
}

impl Config<String> {
    pub fn load_multi_config(path: impl AsRef<Path>, context: Option<&str>) -> Result<Self> {
        let mut multi_config =
            serde_yaml::from_str::<MultiConfig>(&std::fs::read_to_string(path)?)?;

        let context = context.unwrap_or_else(|| &multi_config.default_context);
        tracing::info!(?context, "loading the configuration");

        let config = multi_config
            .contexts
            .remove(context)
            .ok_or_else(|| anyhow!("could not find the context: {}", context))?;

        let config = config.with_context(context.to_owned());
        Ok(config)
    }

    /// Creates a Walrus client with the configuration from `self`.
    pub fn walrus_client(&self) -> Walrus {
        Walrus::new(
            self.walrus_binary(),
            self.gas_budget(),
            self.general.rpc_url.clone(),
            self.general.walrus_config.clone(),
            // TODO: should we ever pass None?
            Some(self.context.clone()),
            self.general.wallet.clone(),
        )
    }
}

// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Configuration for the site builder.

use std::{collections::HashMap, path::Path};

use anyhow::{anyhow, bail, Result};
use serde::Deserialize;
use sui_sdk::wallet_context::WalletContext;
use sui_types::base_types::ObjectID;

pub(crate) use crate::{args::GeneralArgs, walrus::Walrus};

/// Configuration for the site builder, complete with separate context for networks.
#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub(crate) enum MultiConfig {
    SingletonConfig(Config),
    MultiConfig {
        contexts: HashMap<String, Config>,
        default_context: String,
    },
}

/// The configuration for the site builder.
#[derive(Deserialize, Debug, Clone)]
pub(crate) struct Config {
    #[serde(default = "default_portal")]
    pub portal: String,
    pub package: ObjectID,
    #[serde(default)]
    pub general: GeneralArgs,
    pub staking_object: Option<ObjectID>,
}

pub(crate) fn default_portal() -> String {
    "wal.app".to_owned()
}

impl Config {
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

    /// Returns the configuration for the given context or the default context if no context is
    /// provided and the config is a multi config.
    pub fn load_from_multi_config(
        path: impl AsRef<Path>,
        context: Option<&str>,
    ) -> Result<(Self, Option<String>)> {
        let multi_config =
            serde_yaml::from_str::<MultiConfig>(&std::fs::read_to_string(path.as_ref())?)?;

        match multi_config {
            MultiConfig::SingletonConfig(config) => {
                if let Some(context) = context {
                    bail!(
                        "cannot specify context when using a singleton config file \
                        (config_filename={}, context={})",
                        path.as_ref().display(),
                        context
                    );
                }
                Ok((config, context.map(|s| s.to_owned())))
            }
            MultiConfig::MultiConfig {
                mut contexts,
                default_context,
            } => {
                let context = context.unwrap_or(&default_context);
                tracing::info!(?context, "loading the multi config");
                Ok((
                    contexts
                        .remove(context)
                        .ok_or_else(|| anyhow!("could not find the context: {}", context))?,
                    Some(context.to_owned()),
                ))
            }
        }
    }

    /// Creates a Walrus client with the configuration from `self`.
    pub fn walrus_client(&self) -> Walrus {
        Walrus::new(
            self.walrus_binary(),
            self.gas_budget(),
            self.general.rpc_url.clone(),
            self.general.walrus_config.clone(),
            self.general.walrus_context.clone(),
            self.general.wallet.clone(),
        )
    }
}

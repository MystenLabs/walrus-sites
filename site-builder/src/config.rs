// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Configuration for the site builder.

use std::{collections::HashMap, path::Path};

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use sui_sdk::wallet_context::WalletContext;
use sui_types::base_types::ObjectID;

pub(crate) use crate::{args::GeneralArgs, walrus::Walrus};

#[cfg(test)]
#[path = "unit_tests/config.tests.rs"]
mod config_tests;

/// Configuration for the site builder, complete with separate context for networks.
#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub(crate) enum MultiConfig {
    // NB: `MultiConfig` must precede `SingletonConfig` — with `#[serde(untagged)]`, serde
    // tries variants in order. `SingletonConfig(Config)` accepts any map (all fields are
    // optional), so it must be last to allow `MultiConfig` (which requires `contexts` +
    // `default_context`) to match first.
    MultiConfig {
        contexts: HashMap<String, Config>,
        default_context: String,
    },
    SingletonConfig(Config),
}

/// The configuration for the site builder.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    #[serde(default = "default_portal")]
    pub portal: String,
    #[serde(default)]
    pub package: Option<ObjectID>,
    #[serde(default)]
    pub general: GeneralArgs,
    #[serde(skip_serializing_if = "Option::is_none")]
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

    /// Returns the package ID.
    ///
    /// The package is either set in the config file or resolved via MVR during
    /// [`Config::load_from_multi_config`].
    pub fn package(&self) -> ObjectID {
        self.package
            .expect("package must be set (either in config or resolved via MVR)")
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
    ///
    /// For singleton configs, `package` must be specified in the config file.
    /// For multi configs, if `package` is not specified, it is resolved via MVR using the
    /// context name (e.g. "testnet", "mainnet") as the network.
    pub async fn load_from_multi_config(
        path: impl AsRef<Path>,
        context: Option<&str>,
    ) -> Result<(Self, Option<String>)> {
        let config_content = std::fs::read_to_string(path.as_ref()).context(format!(
            "could not read site builder config file '{}'",
            path.as_ref().display()
        ))?;
        let multi_config =
            serde_yaml::from_str::<MultiConfig>(&config_content).context(format!(
                "could not parse site builder config file '{}'",
                path.as_ref().display()
            ))?;

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
                if config.package.is_none() {
                    bail!(
                        "the `package` field is required in a singleton config file ({})",
                        path.as_ref().display()
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
                let mut config = contexts.remove(context).ok_or_else(|| {
                    anyhow!(
                        "could not find the context '{}' in site builder config file '{}'",
                        context,
                        path.as_ref().display()
                    )
                })?;
                if config.package.is_none() {
                    let package_id = crate::mvr::resolve_walrus_sites_package(context).await?;
                    config.package = Some(package_id);
                }
                Ok((config, Some(context.to_owned())))
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

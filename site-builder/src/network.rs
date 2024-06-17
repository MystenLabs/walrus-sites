// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{env, path::PathBuf};

use anyhow::{ensure, Context, Result};
use serde::Deserialize;
use sui_keys::keystore::Keystore;
use sui_sdk::{
    sui_client_config::{SuiClientConfig, SuiEnv},
    SuiClient,
    SuiClientBuilder,
};
use sui_types::base_types::{ObjectID, SuiAddress};

#[derive(Deserialize)]
#[serde(untagged)]
pub enum NetworkConfig {
    Shared(PathBuf),
    Static {
        alias: String,
        rpc: Option<String>,
        address: SuiAddress,
        keystore: Keystore,
    },
}

impl NetworkConfig {
    pub fn alias(&self) -> &str {
        let Self::Static { alias, .. } = self else {
            panic!("expected a static config");
        };

        alias
    }

    pub fn rpc(&self) -> Option<&str> {
        let Self::Static { rpc, .. } = self else {
            panic!("expected a static config");
        };

        rpc.as_ref().map(String::as_str)
    }

    pub fn address(&self) -> SuiAddress {
        let Self::Static { address, .. } = self else {
            panic!("expected a static config");
        };

        *address
    }

    pub fn keystore(&self) -> &Keystore {
        let Self::Static { keystore, .. } = self else {
            panic!("expected a static config");
        };

        keystore
    }

    pub fn load(&mut self) -> Result<&mut Self> {
        let Self::Shared(ref path) = self else {
            return Ok(self);
        };

        let config: SuiClientConfig = std::fs::read_to_string(path)
            .context(format!("unable to load sui config file: {:?}", path))
            .and_then(|s| {
                serde_yaml::from_str(&s).context(format!("unable to parse yaml file: {:?}", path))
            })?;
        let SuiEnv { alias, rpc, .. } = config.get_active_env()?;

        *self = Self::Static {
            alias: alias.to_string(),
            rpc: Some(rpc.to_string()),
            keystore: config.keystore,
            address: config
                .active_address
                .ok_or_else(|| anyhow::anyhow!("active address is not set in sui config"))?,
        };

        Ok(self)
    }

    pub async fn get_sui_client(&self) -> Result<SuiClient> {
        ensure!(
            matches!(self, Self::Static { .. }),
            "config should already be loaded"
        );

        if let Some(ref url) = self.rpc() {
            return Ok(SuiClientBuilder::default().build(url).await?);
        }

        match self.alias() {
            "local" => Ok(SuiClientBuilder::default().build_localnet().await?),
            "devnet" => Ok(SuiClientBuilder::default().build_devnet().await?),
            "testnet" => Ok(SuiClientBuilder::default().build_testnet().await?),
            _ => panic!("expected that only valid aliases are stored"),
        }
    }

    pub fn explorer_url(&self, object: &ObjectID) -> Option<String> {
        if matches!(self.alias(), "local" | "devnet" | "testnet" | "mainnet") {
            let url = format!(
                "https://suiexplorer.com/object/{}?network={}",
                object,
                self.alias()
            );
            Some(url)
        } else {
            None
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        let path =
            PathBuf::from(env::var("HOME").expect("environment variable HOME should be set"))
                .join(".sui")
                .join("sui_config")
                .join("client.yaml");
        Self::Shared(path)
    }
}

impl std::fmt::Debug for NetworkConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Shared(arg0) => f.debug_tuple("Shared").field(arg0).finish(),
            Self::Static {
                alias,
                rpc,
                address,
                keystore: _,
            } => f
                .debug_struct("Static")
                .field("alias", alias)
                .field("rpc", rpc)
                .field("address", address)
                .finish(),
        }
    }
}

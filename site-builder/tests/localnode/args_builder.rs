// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;

use site_builder::args::{Args, Commands, GeneralArgs};
use sui_types::base_types::{ObjectID, SuiAddress};
use thiserror::Error;

pub mod publish_options_builder;
#[allow(unused_imports)]
pub use publish_options_builder::PublishOptionsBuilder;

#[derive(Debug, Clone, Default)]
pub struct ArgsBuilder {
    pub config: Option<PathBuf>,
    /// The context with which to load the configuration.
    ///
    /// If specified, the context will be taken from the config file. Otherwise, the default
    /// context, which is also specified in the config file, will be used.
    pub context: Option<String>,
    pub general: GeneralArgs,
    pub command: Option<Commands>,
}

#[derive(Debug, Error)]
pub enum InvalidArgsConfig {
    #[error("ArgsInner need a command. Try using `.with_command(Commands::...)`.")]
    MissingCommand,
}

impl ArgsBuilder {
    pub fn build(self) -> Result<Args, InvalidArgsConfig> {
        let ArgsBuilder {
            config,
            context,
            general,
            command,
        } = self;
        let Some(command) = command else {
            return Err(InvalidArgsConfig::MissingCommand);
        };

        Ok(Args {
            config,
            context,
            general,
            command,
        })
    }

    pub fn with_config(mut self, config: Option<PathBuf>) -> Self {
        self.config = config;
        self
    }

    pub fn with_context(mut self, context: Option<String>) -> Self {
        self.context = context;
        self
    }

    pub fn with_rpc_url(mut self, rpc_url: Option<String>) -> Self {
        self.general.rpc_url = rpc_url;
        self
    }

    pub fn with_wallet(mut self, wallet: Option<PathBuf>) -> Self {
        self.general.wallet = wallet;
        self
    }

    pub fn with_wallet_env(mut self, wallet_env: Option<String>) -> Self {
        self.general.wallet_env = wallet_env;
        self
    }

    pub fn with_wallet_address(mut self, wallet_address: Option<SuiAddress>) -> Self {
        self.general.wallet_address = wallet_address;
        self
    }

    pub fn with_walrus_context(mut self, walrus_context: Option<String>) -> Self {
        self.general.walrus_context = walrus_context;
        self
    }

    pub fn with_walrus_binary(mut self, walrus_binary: String) -> Self {
        self.general.walrus_binary = Some(walrus_binary);
        self
    }

    pub fn with_walrus_config(mut self, walrus_config: Option<PathBuf>) -> Self {
        self.general.walrus_config = walrus_config;
        self
    }

    pub fn with_walrus_package(mut self, walrus_package: Option<ObjectID>) -> Self {
        self.general.walrus_package = walrus_package;
        self
    }

    pub fn with_gas_budget(mut self, gas_budget: u64) -> Self {
        self.general.gas_budget = Some(gas_budget);
        self
    }

    pub fn with_command(mut self, command: Commands) -> Self {
        self.command.replace(command);
        self
    }
}

// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;

use site_builder::args::{Args, Commands, GeneralArgs};
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
    pub json: bool,
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
            json,
        } = self;
        let Some(command) = command else {
            return Err(InvalidArgsConfig::MissingCommand);
        };

        Ok(Args {
            config,
            context,
            general,
            command,
            json,
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

    pub fn with_general(mut self, general: GeneralArgs) -> Self {
        self.general = general;
        self
    }

    pub fn with_command(mut self, command: Commands) -> Self {
        self.command.replace(command);
        self
    }
}

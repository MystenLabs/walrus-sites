// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! The representation of a walrus cli command.

use std::{num::NonZeroU16, path::PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

use super::types::BlobId;

/// Represents a call to the JSON mode of the Walrus CLI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalrusJsonCmd {
    /// The path to the configuration file for the Walrus CLI.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<PathBuf>,
    /// The path for the wallet to use with Walrus.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet: Option<PathBuf>,
    /// The gas budget for the transaction.
    #[serde(default = "default::gas_budget")]
    pub gas_budget: u64,
    /// The command to be run.
    pub command: Command,
}

impl WalrusJsonCmd {
    /// Serializes the command to json.
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self)?)
    }
}

/// Represents a command to be run on the Walrus CLI.
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Command {
    /// Stores a blob to Walrus.
    Store {
        /// The path to the file to be stored.
        file: PathBuf,
        /// The number of epochs for which to store the file.
        #[serde(default = "default::epochs")]
        epochs: u64,
        /// Do not check for the blob status before storing it.
        ///
        /// This will create a new blob even if the blob is already certified for a sufficient
        /// duration.
        #[serde(default)]
        force: bool,
    },
    /// Reads a blob from Walrus.
    Read {
        /// The blob ID of the blob to be read.
        #[serde_as(as = "DisplayFromStr")]
        blob_id: BlobId,
        /// The optional path to which the blob should be saved.
        #[serde(default)]
        #[serde(skip_serializing_if = "Option::is_none")]
        out: Option<PathBuf>,
        /// The RPC endpoint to which the Walrus CLI should connect to.
        #[serde(default)]
        rpc_arg: RpcArg,
    },
    BlobId {
        file: PathBuf,
        /// The number of shards of the Walrus system.
        ///
        /// If specified, the CLI will compute the blob ID without connecting to Sui. Otherwise, it
        /// will connect to the chain to read the committee.
        #[serde(skip_serializing_if = "Option::is_none")]
        n_shards: Option<NonZeroU16>,
        /// The RPC endpoint to which the Walrus CLI should connect to.
        #[serde(default)]
        #[serde(skip_serializing_if = "RpcArg::is_none")]
        rpc_arg: RpcArg,
    },
}

/// Represents the Sui RPC endpoint argument.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcArg {
    /// The RPC URL of a Sui full node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rpc_url: Option<String>,
}

impl RpcArg {
    /// Checks if the inner RPC URL is `None`.
    fn is_none(&self) -> bool {
        self.rpc_url.is_none()
    }
}

mod default {
    pub(crate) fn gas_budget() -> u64 {
        500_000_000
    }

    pub(crate) fn epochs() -> u64 {
        1
    }
}

/// Helper struct to build [`WalrusJsonCmd`] instances.
#[derive(Debug, Clone)]
pub struct WalrusCmdBuilder<T = ()> {
    config: Option<PathBuf>,
    wallet: Option<PathBuf>,
    gas_budget: u64,
    command: T,
}

impl WalrusCmdBuilder {
    /// Creates a new builder.
    pub fn new(config: Option<PathBuf>, wallet: Option<PathBuf>, gas_budget: u64) -> Self {
        Self {
            config,
            wallet,
            gas_budget,
            command: (),
        }
    }

    /// Adds a [`Command`] to the builder.
    pub fn with_command(self, command: Command) -> WalrusCmdBuilder<Command> {
        let Self {
            config,
            wallet,
            gas_budget,
            ..
        } = self;
        WalrusCmdBuilder {
            config,
            wallet,
            gas_budget,
            command,
        }
    }

    /// Adds a [`Command::Store`] command to the builder.
    pub fn store(self, file: PathBuf, epochs: u64, force: bool) -> WalrusCmdBuilder<Command> {
        let command = Command::Store {
            file,
            epochs,
            force,
        };
        self.with_command(command)
    }

    /// Adds a [`Command::Read`] command to the builder.
    #[allow(dead_code)]
    pub fn read(
        self,
        blob_id: BlobId,
        out: Option<PathBuf>,
        rpc_arg: RpcArg,
    ) -> WalrusCmdBuilder<Command> {
        let command = Command::Read {
            blob_id,
            out,
            rpc_arg,
        };
        self.with_command(command)
    }

    /// Adds a [`Command::BlobId`] command to the builder.
    pub fn blob_id(
        self,
        file: PathBuf,
        n_shards: Option<NonZeroU16>,
        rpc_arg: RpcArg,
    ) -> WalrusCmdBuilder<Command> {
        let command = Command::BlobId {
            file,
            n_shards,
            rpc_arg,
        };
        self.with_command(command)
    }
}

impl WalrusCmdBuilder<Command> {
    /// Builds the [`WalrusJsonCmd`] by consuming the builder.
    pub fn build(self) -> WalrusJsonCmd {
        let WalrusCmdBuilder {
            config,
            wallet,
            gas_budget,
            command,
        } = self;
        WalrusJsonCmd {
            config,
            wallet,
            gas_budget,
            command,
        }
    }
}

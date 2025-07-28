// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! The representation of a walrus cli command.

use std::{collections::BTreeMap, num::NonZeroU16, path::PathBuf};

use anyhow::Result;
use clap::Args;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

use super::types::BlobId;
use crate::args::EpochArg;

/// Represents a call to the JSON mode of the Walrus CLI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalrusJsonCmd {
    /// The path to the configuration file for the Walrus CLI.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<PathBuf>,
    /// The configuration context to use for the client, if omitted the default_context is used.
    #[serde(default)]
    pub context: Option<String>,
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
        /// The files containing the blob to be published to Walrus.
        files: Vec<PathBuf>,
        /// Common options shared between store and store-quilt commands.
        #[serde(flatten)]
        common_options: CommonStoreOptions,
    },
    /// Store files as a quilt.
    StoreQuilt {
        /// Paths to files to include in the quilt.
        ///
        /// If a path is a directory, all the files in the directory will be included
        /// in the quilt, recursively.
        /// If a path is a file, the file will be included in the quilt.
        /// The filenames are used as the identifiers of the quilt patches.
        /// Note duplicate filenames are not allowed.
        /// Custom identifiers and tags are NOT supported for quilt patches.
        /// Use `--blobs` to specify custom identifiers and tags.
        paths: Vec<PathBuf>,
        /// Blobs to include in the quilt, each blob is specified as a JSON string.
        ///
        /// Example:
        ///   walrus store-quilt --epochs 10
        ///     --blobs '{"path":"/path/to/food-locations.pdf","identifier":"paper-v2",\
        ///     "tags":{"author":"Walrus","project":"food","status":"final-review"}}' \
        ///     '{"path":"/path/to/water-locations.pdf","identifier":"water-v3",\
        ///     "tags":{"author":"Walrus","project":"water","status":"draft"}}'
        /// Note if identifier is not specified, the filename will be used as the identifier,
        /// and duplicate identifiers are not allowed.
        #[serde(default)]
        blobs: Vec<QuiltBlobInput>,
        /// Common options shared between store and store-quilt commands.
        #[serde(flatten)]
        common_options: CommonStoreOptions,
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
    /// Deletes a blob from Walrus.
    Delete {
        /// The the blob ID(s) of the blob(s) to delete.
        #[serde_as(as = "Vec<DisplayFromStr>")]
        blob_ids: Vec<BlobId>,
        /// Disable checking the status of the blob after deletion.
        ///
        /// Checking the status adds delay and requires additional requests.
        #[serde(default)]
        no_status_check: bool,
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
        #[serde(flatten)]
        rpc_arg: RpcArg,
    },
    Info {
        /// The URL of the Sui RPC node to use.
        #[serde(flatten)]
        rpc_arg: RpcArg,
        /// The specific info command to run.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        command: Option<InfoCommands>,
    },
}

/// Represents a blob to be stored in a quilt, together with its identifier and tags.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct QuiltBlobInput {
    /// The path to the blob.
    pub(crate) path: PathBuf,
    /// The identifier of the blob.
    ///
    /// If not provided, the file name will be used as the identifier.
    #[serde(default)]
    pub(crate) identifier: Option<String>,
    /// The tags of the blob.
    #[serde(default)]
    pub(crate) tags: BTreeMap<String, String>,
}

/// Common options shared between store and store-quilt commands.
#[derive(Debug, Clone, Args, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommonStoreOptions {
    /// The epoch argument to specify either the number of epochs to store the blob, or the
    /// end epoch, or the earliest expiry time in rfc3339 format.
    #[command(flatten)]
    #[serde(flatten)]
    pub epoch_arg: EpochArg,
    /// Perform a dry-run of the store without performing any actions on chain.
    ///
    /// This assumes `--force`; i.e., it does not check the current status of the blob/quilt.
    #[arg(long)]
    #[serde(default)]
    pub dry_run: bool,
    /// Do not check for the blob/quilt status before storing it.
    ///
    /// This will create a new blob/quilt even if it is already certified for a sufficient
    /// duration.
    #[arg(long)]
    #[serde(default)]
    pub force: bool,
    /// Ignore the storage resources owned by the wallet.
    ///
    /// The client will not check if it can reuse existing resources, and just check the blob/quilt
    /// status on chain.
    #[arg(long)]
    #[serde(default)]
    pub ignore_resources: bool,
    /// Mark the blob/quilt as deletable.
    ///
    /// Deletable blobs/quilts can be removed from Walrus before their expiration time.
    ///
    /// *This will become the default behavior in the future.*
    #[serde(default)]
    pub deletable: bool,
    /// Whether to put the blob/quilt into a shared object.
    #[arg(long)]
    #[serde(default)]
    pub share: bool,
}

/// Represents the Sui RPC endpoint argument.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcArg {
    /// The RPC URL of a Sui full node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rpc_url: Option<String>,
}

/// Subcommands for the `info` command.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum InfoCommands {
    /// Print all information listed below.
    All,
    /// Print epoch information.
    Epoch,
    /// Print storage information.
    Storage,
    /// Print size information.
    Size,
    /// Print price information.
    Price,
    /// Print byzantine fault tolerance (BFT) information.
    Bft,
    /// Print committee information.
    Committee,
}

mod default {
    pub(crate) fn gas_budget() -> u64 {
        500_000_000
    }
}

/// Helper struct to build [`WalrusJsonCmd`] instances.
#[derive(Debug, Clone)]
pub struct WalrusCmdBuilder<T = ()> {
    config: Option<PathBuf>,
    context: Option<String>,
    wallet: Option<PathBuf>,
    gas_budget: u64,
    command: T,
}

impl WalrusCmdBuilder {
    /// Creates a new builder.
    pub fn new(
        config: Option<PathBuf>,
        context: Option<String>,
        wallet: Option<PathBuf>,
        gas_budget: u64,
    ) -> Self {
        Self {
            config,
            context,
            wallet,
            gas_budget,
            command: (),
        }
    }

    /// Adds a [`Command`] to the builder.
    pub fn with_command(self, command: Command) -> WalrusCmdBuilder<Command> {
        let Self {
            config,
            context,
            wallet,
            gas_budget,
            ..
        } = self;
        WalrusCmdBuilder {
            config,
            context,
            wallet,
            gas_budget,
            command,
        }
    }

    /// Adds a [`Command::Store`] command to the builder.
    pub fn store(
        self,
        files: Vec<PathBuf>,
        common_store_options: CommonStoreOptions,
    ) -> WalrusCmdBuilder<Command> {
        let command = Command::Store {
            files,
            common_options: common_store_options,
        };
        self.with_command(command)
    }

    /// Adds a [`Command::StoreQuilt`] command to the builder.
    pub fn store_quilt(
        self,
        paths: Vec<PathBuf>,
        blobs: Vec<QuiltBlobInput>,
        common_store_options: CommonStoreOptions,
    ) -> WalrusCmdBuilder<Command> {
        let command = Command::StoreQuilt {
            paths,
            blobs,
            common_options: common_store_options,
        };
        self.with_command(command)
    }

    /// Adds a [`Command::Delete`] command to the builder.
    pub fn delete(self, blob_ids: &[BlobId]) -> WalrusCmdBuilder<Command> {
        let command = Command::Delete {
            blob_ids: blob_ids.to_vec(),
            no_status_check: false,
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

    /// Adds an [`Command::Info`] command to the builder.
    pub fn info(
        self,
        rpc_arg: RpcArg,
        subcommand: Option<InfoCommands>,
    ) -> WalrusCmdBuilder<Command> {
        let command = Command::Info {
            rpc_arg,
            command: subcommand,
        };
        self.with_command(command)
    }
}

impl WalrusCmdBuilder<Command> {
    /// Builds the [`WalrusJsonCmd`] by consuming the builder.
    pub fn build(self) -> WalrusJsonCmd {
        let WalrusCmdBuilder {
            config,
            context,
            wallet,
            gas_budget,
            command,
        } = self;
        WalrusJsonCmd {
            config,
            context,
            wallet,
            gas_budget,
            command,
        }
    }
}

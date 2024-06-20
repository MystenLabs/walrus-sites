// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! High-level controller for the Walrus binary through the JSON interface.

use std::{num::NonZeroU16, path::PathBuf, process::Command as CliCommand};

use anyhow::{Context, Result};
use command::RpcArg;
use output::{try_from_output, BlobIdOutput, ReadOutput, StoreOutput};

use self::types::BlobId;
use crate::walrus::command::WalrusCmdBuilder;

pub mod command;
pub mod output;
pub mod types;

/// Controller to execute actions on Walrus.
#[derive(Debug, Clone)]
pub struct Walrus {
    /// The name of the Walrus binary.
    bin: String,
    /// The gas budget for Walrus transactions.
    gas_budget: u64,
    /// The RPC url the Walrus CLI will use in the calls to Sui.
    rpc_url: Option<String>,
    /// The path to the Walrus cli Config.
    config: Option<PathBuf>,
    /// The path to the Sui Wallet config.
    wallet: Option<PathBuf>,
}

macro_rules! create_command {
    ($self:ident, $name:ident, $($arg:expr),*) => {{
        let json_input = $self.builder().$name($($arg),*).build().to_json()?;
        let output = $self
            .base_command()
            .arg(&json_input)
            .output()
            .context(
                format!(
                    "error while executing the call to the Walrus binary; \
                    is it available and executable? you are using: `{}`",
                    $self.bin
                )
            )?;
        try_from_output(output)
    }};
}

impl Walrus {
    /// Creates a new Walrus CLI controller.
    pub fn new(
        bin: String,
        gas_budget: u64,
        rpc_url: Option<String>,
        config: Option<PathBuf>,
        wallet: Option<PathBuf>,
    ) -> Self {
        Self {
            bin,
            gas_budget,
            rpc_url,
            config,
            wallet,
        }
    }

    /// Issues a `store` JSON command to the Walrus CLI, returning the parsed output.
    pub fn store(&self, file: PathBuf, epochs: u64, force: bool) -> Result<StoreOutput> {
        create_command!(self, store, file, epochs, force)
    }

    // TODO(giac): currently blocking. Parallelize reads.
    /// Issues a `read` JSON command to the Walrus CLI, returning the parsed output.
    #[allow(dead_code)]
    pub fn read(&self, blob_id: BlobId, out: Option<PathBuf>) -> Result<ReadOutput> {
        create_command!(self, read, blob_id, out, self.rpc_arg())
    }

    // TODO(giac): currently blocking. Parallelize reads.
    // TODO(giac): maybe preconfigure the `n_shards` to avid repeating `None`.
    /// Issues a `blob_id` JSON command to the Walrus CLI, returning the parsed output.
    pub fn blob_id(&self, file: PathBuf, n_shards: Option<NonZeroU16>) -> Result<BlobIdOutput> {
        create_command!(self, blob_id, file, n_shards, self.rpc_arg())
    }

    fn base_command(&self) -> CliCommand {
        let mut cmd = CliCommand::new(&self.bin);
        cmd.arg("json");
        cmd
    }

    fn builder(&self) -> WalrusCmdBuilder {
        WalrusCmdBuilder::new(self.config.clone(), self.wallet.clone(), self.gas_budget)
    }

    fn rpc_arg(&self) -> RpcArg {
        RpcArg {
            rpc_url: self.rpc_url.clone(),
        }
    }
}

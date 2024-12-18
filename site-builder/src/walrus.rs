// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! High-level controller for the Walrus binary through the JSON interface.

use std::{num::NonZeroU16, path::PathBuf};

use anyhow::{Context, Result};
use command::RpcArg;
use output::{
    try_from_output,
    BlobIdOutput,
    DryRunOutput,
    InfoOutput,
    NShards,
    ReadOutput,
    StoreOutput,
};
use tokio::process::Command as CliCommand;

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
            .await
            .context(
                format!(
                    "error while executing the call to the Walrus binary; \
                    is it available and executable? you are using: `{}`",
                    $self.bin
                )
            )?;
        try_from_output(output)
            .inspect(|output| tracing::debug!(?output, "Walrus CLI parsed output"))
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
    // NOTE: takes a mutable reference to ensure that only one store command is executed at every
    // time. The issue is that the inner wallet may lock coins if called in parallel.
    pub async fn store(&mut self, file: PathBuf, epochs: u64, force: bool) -> Result<StoreOutput> {
        create_command!(self, store, file, epochs, force, false)
    }

    /// Issues a `store with dry run arg` JSON command to the Walrus CLI, returning the parsed output.
    // NOTE: takes a mutable reference to ensure that only one store command is executed at every
    // time. The issue is that the inner wallet may lock coins if called in parallel.
    pub async fn dry_run_store(
        &mut self,
        file: PathBuf,
        epochs: u64,
        force: bool,
    ) -> Result<DryRunOutput> {
        create_command!(self, store, file, epochs, force, true)
    }

    /// Issues a `read` JSON command to the Walrus CLI, returning the parsed output.
    #[allow(dead_code)]
    pub async fn read(&self, blob_id: BlobId, out: Option<PathBuf>) -> Result<ReadOutput> {
        create_command!(self, read, blob_id, out, self.rpc_arg())
    }

    // TODO(giac): maybe preconfigure the `n_shards` to avid repeating `None`.
    /// Issues a `blob_id` JSON command to the Walrus CLI, returning the parsed output.
    pub async fn blob_id(
        &self,
        file: PathBuf,
        n_shards: Option<NonZeroU16>,
    ) -> Result<BlobIdOutput> {
        create_command!(self, blob_id, file, n_shards, self.rpc_arg())
    }

    /// Issues an `info` JSON command to the Walrus CLI, returning the parsed output.
    #[allow(dead_code)]
    pub async fn info(&self, dev: bool) -> Result<InfoOutput> {
        create_command!(self, info, self.rpc_arg(), dev)
    }

    /// Issues an `info` JSON command to the Walrus CLI, returning the number of shards.
    pub async fn n_shards(&self) -> Result<NonZeroU16> {
        let n_shards: NShards = create_command!(self, info, self.rpc_arg(), false)?;
        Ok(n_shards.n_shards)
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

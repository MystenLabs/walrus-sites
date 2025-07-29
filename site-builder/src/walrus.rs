// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! High-level controller for the Walrus binary through the JSON interface.

use std::{num::NonZeroU16, path::PathBuf};

use anyhow::{Context, Result};
use command::{InfoCommands, RpcArg};
use output::{
    try_from_output,
    BlobIdOutput,
    DryRunOutput,
    ReadOutput,
    StorageInfoOutput,
    StoreOutput,
};
use tokio::process::Command as CliCommand;

use self::types::BlobId;
use crate::{
    args::EpochArg,
    walrus::{
        command::{CommonStoreOptions, QuiltBlobInput, WalrusCmdBuilder},
        output::{DestroyOutput, QuiltStoreResult, StoreQuiltDryRunOutput},
    },
};
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
    /// The context to use for the Walrus CLI.
    context: Option<String>,
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
        context: Option<String>,
        wallet: Option<PathBuf>,
    ) -> Self {
        Self {
            bin,
            gas_budget,
            rpc_url,
            config,
            context,
            wallet,
        }
    }

    /// Issues a `store` JSON command to the Walrus CLI, returning the parsed output.
    // NOTE: takes a mutable reference to ensure that only one store command is executed at every
    // time. The issue is that the inner wallet may lock coins if called in parallel.
    pub async fn store(
        &mut self,
        files: Vec<PathBuf>,
        epoch_arg: EpochArg,
        force: bool,
        deletable: bool,
    ) -> Result<StoreOutput> {
        let dry_run = false;
        let ignore_resources = false;
        let share = false;
        create_command!(
            self,
            store,
            files,
            CommonStoreOptions {
                epoch_arg,
                dry_run,
                force,
                ignore_resources,
                deletable,
                share,
            }
        )
    }

    /// Issues a `store-quilt` JSON command to the Walrus CLI, returning the parsed output.
    pub async fn store_quilt(
        &mut self,
        paths: Vec<PathBuf>,
        blobs: Vec<QuiltBlobInput>,
        epoch_arg: EpochArg,
        force: bool,
        deletable: bool,
    ) -> Result<QuiltStoreResult> {
        let dry_run = false;
        let ignore_resources = false;
        let share = false;
        create_command!(
            self,
            store_quilt,
            paths,
            blobs,
            CommonStoreOptions {
                epoch_arg,
                dry_run,
                force,
                ignore_resources,
                deletable,
                share,
            }
        )
    }

    /// Issues a `delete` JSON command to the Walrus CLI, returning the parsed output.
    pub async fn delete(&mut self, blob_ids: &[BlobId]) -> Result<Vec<DestroyOutput>> {
        create_command!(self, delete, blob_ids)
    }

    /// Issues a `store with dry run arg` JSON command to the Walrus CLI, returning the parsed output.
    pub async fn dry_run_store(
        &mut self,
        file: PathBuf,
        epoch_arg: EpochArg,
        deletable: bool,
        force: bool,
    ) -> Result<Vec<DryRunOutput>> {
        let dry_run = true;
        let ignore_resources = false;
        let share = false;
        create_command!(
            self,
            store,
            vec![file],
            CommonStoreOptions {
                epoch_arg,
                dry_run,
                force,
                ignore_resources,
                deletable,
                share,
            }
        )
    }

    /// Issues a `dry_run_store_quilt` JSON command to the Walrus CLI, returning the parsed output.
    pub async fn dry_run_store_quilt(
        &mut self,
        path_or_blob: PathOrBlob,
        epoch_arg: EpochArg,
        force: bool,
        deletable: bool,
    ) -> Result<Vec<StoreQuiltDryRunOutput>> {
        let dry_run = true;
        let ignore_resources = false;
        let share = false;
        // The `paths` and `blobs` are in conflict with each other
        let (paths, blobs) = match path_or_blob {
            PathOrBlob::Path(path) => (vec![path], Vec::new()),
            PathOrBlob::Blob(blob) => (Vec::new(), vec![blob]),
        };
        create_command!(
            self,
            store_quilt,
            paths,
            blobs,
            CommonStoreOptions {
                epoch_arg,
                dry_run,
                force,
                ignore_resources,
                deletable,
                share,
            }
        )
    }

    /// Issues a `read` JSON command to the Walrus CLI, returning the parsed output.
    #[allow(dead_code)]
    pub async fn read(&self, blob_id: BlobId, out: Option<PathBuf>) -> Result<ReadOutput> {
        create_command!(self, read, blob_id, out, self.rpc_arg())
    }

    /// Issues a `blob_id` JSON command to the Walrus CLI, returning the parsed output.
    pub async fn blob_id(
        &self,
        file: PathBuf,
        n_shards: Option<NonZeroU16>,
    ) -> Result<BlobIdOutput> {
        create_command!(self, blob_id, file, n_shards, self.rpc_arg())
    }

    /// Issues an `info` JSON command to the Walrus CLI, returning the number of shards.
    pub async fn n_shards(&self) -> Result<NonZeroU16> {
        let n_shards: StorageInfoOutput =
            create_command!(self, info, self.rpc_arg(), Some(InfoCommands::Storage))?;
        Ok(n_shards.n_shards)
    }

    fn base_command(&self) -> CliCommand {
        let mut cmd = CliCommand::new(&self.bin);
        cmd.arg("json");
        cmd
    }

    fn builder(&self) -> WalrusCmdBuilder {
        WalrusCmdBuilder::new(
            self.config.clone(),
            self.context.clone(),
            self.wallet.clone(),
            self.gas_budget,
        )
    }

    fn rpc_arg(&self) -> RpcArg {
        RpcArg {
            rpc_url: self.rpc_url.clone(),
        }
    }
}

pub enum PathOrBlob {
    Path(PathBuf),
    Blob(QuiltBlobInput),
}

// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Arguments for the site builder CLI.

use std::{
    num::{NonZeroU32, NonZeroUsize},
    path::PathBuf,
};

use anyhow::{anyhow, ensure, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use sui_sdk::wallet_context::WalletContext;
use sui_types::base_types::ObjectID;

use crate::{util::load_wallet_context, walrus::output::EpochCount};

#[derive(Parser, Clone, Debug, Deserialize)]
#[clap(rename_all = "kebab-case")]
pub(crate) struct GeneralArgs {
    /// The URL or the RPC endpoint to connect the client to.
    ///
    /// Can be specified as a CLI argument or in the config.
    #[clap(long)]
    pub(crate) rpc_url: Option<String>,
    /// The path to the Sui Wallet config.
    ///
    /// Can be specified as a CLI argument or in the config.
    #[clap(long)]
    pub(crate) wallet: Option<PathBuf>,
    /// The env to be used for the Sui wallet.
    ///
    /// If not specified, the env specified in the sites-config (under `wallet_env`) will be used.
    /// If the wallet env is also not specified in the config, the env configured in the Sui client
    /// will be used.
    #[clap(long)]
    pub(crate) wallet_env: Option<String>,
    /// The path or name of the walrus binary.
    ///
    /// The Walrus binary will then be called with this configuration to perform actions on Walrus.
    /// Can be specified as a CLI argument or in the config.
    #[clap(long)]
    #[serde(default = "default::walrus_binary")]
    pub(crate) walrus_binary: Option<String>,
    /// The path to the configuration for the Walrus client.
    ///
    /// This will be passed to the calls to the Walrus binary.
    /// Can be specified as a CLI argument or in the config.
    #[clap(long)]
    pub(crate) walrus_config: Option<PathBuf>,
    /// The gas budget for the operations on Sui.
    ///
    /// Can be specified as a CLI argument or in the config.
    #[clap(long)]
    #[clap(short, long)]
    #[serde(default = "default::gas_budget")]
    pub(crate) gas_budget: Option<u64>,
}

impl Default for GeneralArgs {
    fn default() -> Self {
        Self {
            rpc_url: None,
            wallet: None,
            wallet_env: None,
            walrus_binary: default::walrus_binary(),
            walrus_config: None,
            gas_budget: default::gas_budget(),
        }
    }
}

impl GeneralArgs {
    /// Returns the wallet context from the configuration.
    ///
    /// If no wallet is specified, the default wallet will be used.
    pub fn load_wallet(&self) -> Result<WalletContext> {
        load_wallet_context(self.wallet.as_deref(), self.wallet_env.as_deref())
    }
}

macro_rules! merge {
    ($self:ident, $other:ident, $field:ident) => {
        if $other.$field.is_some() {
            $self.$field = $other.$field.clone();
        }
    };
}

macro_rules! merge_fields {
    ($self:ident, $other:ident, $($field:ident),* $(,)?) => (
        $(
            merge!($self, $other, $field);
        )*
    );
}

impl GeneralArgs {
    /// Merges two instances of [`GeneralArgs`], keeping all the `Some` values.
    ///
    /// The values of `other` are taken before the values of `self`.
    pub fn merge(&mut self, other: &Self) {
        merge_fields!(
            self,
            other,
            rpc_url,
            wallet,
            walrus_binary,
            walrus_config,
            gas_budget,
        );
    }
}

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case")]
pub(crate) enum Commands {
    /// Publish a new site on Sui.
    Publish {
        #[clap(flatten)]
        publish_options: PublishOptions,
        /// The name of the site.
        #[clap(short, long, default_value = "test site")]
        site_name: String,
    },
    /// Update an existing site.
    Update {
        #[clap(flatten)]
        publish_options: PublishOptions,
        /// The object ID of a partially published site to be completed.
        object_id: ObjectID,
        #[clap(short, long, action)]
        watch: bool,
        /// Publish all resources to Sui and Walrus, even if they may be already present.
        ///
        /// This can be useful in case the Walrus devnet is reset, but the resources are still
        /// available on Sui.
        #[clap(long, action)]
        force: bool,
    },
    /// Convert an object ID in hex format to the equivalent Base36 format.
    ///
    /// This command may be useful to browse a site, given it object ID.
    Convert {
        /// The object id (in hex format) to convert
        object_id: ObjectID,
    },
    /// Show the pages composing the site at the given object ID.
    Sitemap { object: ObjectID },
    /// Preprocess the directory, creating and linking index files.
    /// This command allows to publish directories as sites. Warning: Rewrites all `index.html`
    /// files.
    ListDirectory { path: PathBuf },
    /// Completely destroys the site at the given object id.
    ///
    /// Removes all resources and routes, and destroys the site, returning the Sui storage rebate to
    /// the owner. Warning: this action is irreversible! Re-publishing the site will generate a
    /// different Site object ID.
    Destroy { object: ObjectID },
    /// Adds or updates a single resource in a site, eventually replacing any pre-existing ones.
    ///
    /// The ws_resource file will still be used to determine the resource's headers.
    UpdateResource {
        /// The path to the resource to be added.
        #[clap(long)]
        resource: PathBuf,
        /// The path the resource should have in the site.
        ///
        /// Should be in the form `/path/to/resource.html`, with a leading `/`.
        #[clap(long)]
        path: String,
        /// The object ID of the Site object on Sui, to which the resource will be added.
        #[clap(long)]
        site_object: ObjectID,
        /// The path to the Walrus sites resources file.
        ///
        /// This JSON configuration file defined HTTP resource headers and other utilities for your
        /// files. By default, the file is expected to be named `ws-resources.json` and located in the
        /// root of the site directory.
        ///
        /// The configuration file _will not_ be uploaded to Walrus.
        #[clap(long)]
        // TODO: deduplicate with the `publish_options` in the `Publish` and `Update` commands.
        ws_resources: Option<PathBuf>,
        /// The number of epochs for which to save the resources on Walrus.
        ///
        /// If set to `max`, the resources are stored for the maximum number of epochs allowed on
        /// Walrus. Otherwise, the resources are stored for the specified number of epochs. The
        /// number of epochs must be greater than 0.
        #[clap(long, value_parser = EpochCountOrMax::parse_epoch_count)]
        epochs: EpochCountOrMax,
        /// By default, sites are deletable with site-builder delete command. By passing --permanent, the site is deleted only after `epochs` expiration.
        /// Make resources permanent (non-deletable)
        #[clap(long, action = clap::ArgAction::SetTrue)]
        permanent: bool,
        /// Perform a dry run (you'll be asked for confirmation before committing changes).
        #[clap(long)]
        dry_run: bool,
    },
}

#[derive(Parser, Debug, Clone)]
pub(crate) struct PublishOptions {
    /// The directory containing the site sources.
    pub(crate) directory: PathBuf,
    /// The path to the Walrus sites resources file.
    ///
    /// This JSON configuration file defined HTTP resource headers and other utilities for your
    /// files. By default, the file is expected to be named `ws-resources.json` and located in the
    /// root of the site directory.
    ///
    /// The configuration file _will not_ be uploaded to Walrus.
    #[clap(long)]
    pub(crate) ws_resources: Option<PathBuf>,
    /// The number of epochs for which to save the resources on Walrus.
    ///
    /// If set to `max`, the resources are stored for the maximum number of epochs allowed on
    /// Walrus. Otherwise, the resources are stored for the specified number of epochs. The
    /// number of epochs must be greater than 0.
    #[clap(long, value_parser = EpochCountOrMax::parse_epoch_count)]
    pub(crate) epochs: EpochCountOrMax,
    /// Preprocess the directory before publishing.
    /// See the `list-directory` command. Warning: Rewrites all `index.html` files.
    #[clap(long, action)]
    pub(crate) list_directory: bool,
    /// The maximum number of concurrent calls to the Walrus CLI for the computation of blob IDs.
    #[clap(long)]
    pub(crate) max_concurrent: Option<NonZeroUsize>,
    /// By default, sites are deletable with site-builder delete command. By passing --permanent, the site is deleted only after `epochs` expiration.
    /// Make resources permanent (non-deletable)
    #[clap(long, action = clap::ArgAction::SetTrue)]
    pub(crate) permanent: bool,
    /// Perform a dry run (you'll be asked for confirmation before committing changes).
    #[clap(long)]
    pub(crate) dry_run: bool,
}

/// The number of epochs to store the blobs for.
///
/// Can be either a non-zero number of epochs or the special value `max`, which will store the blobs
/// for the maximum number of epochs allowed by the system object on chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum EpochCountOrMax {
    /// Store the blobs for the maximum number of epochs allowed.
    #[serde(rename = "max")]
    Max,
    /// The number of epochs to store the blobs for.
    #[serde(untagged)]
    Epochs(NonZeroU32),
}

impl EpochCountOrMax {
    fn parse_epoch_count(input: &str) -> Result<Self> {
        if input == "max" {
            Ok(Self::Max)
        } else {
            let epochs = input.parse::<u32>()?;
            Ok(Self::Epochs(NonZeroU32::new(epochs).ok_or_else(|| {
                anyhow!("invalid epoch count; please a number >0 or `max`")
            })?))
        }
    }

    /// Tries to convert the `EpochCountOrMax` into an `EpochCount` value.
    ///
    /// If the `EpochCountOrMax` is `Max`, the `max_epochs_ahead` is used as the maximum number of
    /// epochs that can be stored ahead.
    #[allow(unused)]
    pub fn try_into_epoch_count(&self, max_epochs_ahead: EpochCount) -> anyhow::Result<EpochCount> {
        match self {
            EpochCountOrMax::Max => Ok(max_epochs_ahead),
            EpochCountOrMax::Epochs(epochs) => {
                let epochs = epochs.get();
                ensure!(
                    epochs <= max_epochs_ahead,
                    "blobs can only be stored for up to {} epochs ahead; {} epochs were requested",
                    max_epochs_ahead,
                    epochs
                );
                Ok(epochs)
            }
        }
    }
}

mod default {
    pub(crate) fn walrus_binary() -> Option<String> {
        Some("walrus".to_owned())
    }
    pub(crate) fn gas_budget() -> Option<u64> {
        Some(500_000_000)
    }
}

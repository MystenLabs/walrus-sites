// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Arguments for the site builder CLI.

use std::{
    num::{NonZeroU32, NonZeroUsize},
    path::PathBuf,
    str::FromStr,
    time::SystemTime,
};

use anyhow::{anyhow, ensure, Result};
use clap::{ArgGroup, Parser, Subcommand};
use serde::{Deserialize, Serialize};
use sui_sdk::wallet_context::WalletContext;
use sui_types::base_types::{ObjectID, SuiAddress};

use crate::{
    retry_client::RetriableSuiClient,
    suins::SuiNsClient,
    util::load_wallet_context,
    walrus::output::EpochCount,
};

#[derive(Parser, Debug)]
#[command(rename_all = "kebab-case")]
pub struct Args {
    /// The path to the configuration file for the site builder.
    #[arg(short, long)]
    pub config: Option<PathBuf>,
    /// The context with which to load the configuration.
    ///
    /// If specified, the context will be taken from the config file. Otherwise, the default
    /// context, which is also specified in the config file, will be used.
    #[arg(long)]
    pub context: Option<String>,
    #[clap(flatten)]
    pub general: GeneralArgs,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Parser, Clone, Debug, Deserialize, Serialize)]
#[command(rename_all = "kebab-case")]
pub struct GeneralArgs {
    /// The URL or the RPC endpoint to connect the client to.
    ///
    /// Can be specified as a CLI argument or in the config.
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rpc_url: Option<String>,
    /// The path to the Sui Wallet config.
    ///
    /// Can be specified as a CLI argument or in the config.
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet: Option<PathBuf>,
    /// The env to be used for the Sui wallet.
    ///
    /// If not specified, the env specified in the sites-config (under `wallet_env`) will be used.
    /// If the wallet env is also not specified in the config, the env configured in the Sui client
    /// will be used.
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet_env: Option<String>,
    /// The address to be used for the Sui wallet.
    ///
    /// If not specified, the address specified in the sites-config (under `wallet_address`) will be
    /// used. If the wallet address is also not specified in the config, the address configured in
    /// the Sui client will be used.
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet_address: Option<SuiAddress>,
    /// The context that will be passed to the Walrus binary.
    ///
    /// If not specified, the Walrus context specified in the sites-config will be
    /// used. If it is also not specified in the config, no context will be passed.
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub walrus_context: Option<String>,
    /// The path or name of the walrus binary.
    ///
    /// The Walrus binary will then be called with this configuration to perform actions on Walrus.
    /// Can be specified as a CLI argument or in the config.
    #[arg(long)]
    #[serde(default = "default::walrus_binary")]
    pub walrus_binary: Option<String>,
    /// The path to the configuration for the Walrus client.
    ///
    /// This will be passed to the calls to the Walrus binary.
    /// Can be specified as a CLI argument or in the config.
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub walrus_config: Option<PathBuf>,
    /// The package ID of the Walrus package on the selected network.
    ///
    /// This is currently only used for the `sitemap` command.
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub walrus_package: Option<ObjectID>,
    /// The gas budget for the operations on Sui.
    ///
    /// Can be specified as a CLI argument or in the config.
    #[arg(long)]
    #[serde(default = "default::gas_budget")]
    pub gas_budget: Option<u64>,
}

impl Default for GeneralArgs {
    fn default() -> Self {
        Self {
            rpc_url: None,
            wallet: None,
            wallet_env: None,
            wallet_address: None,
            walrus_context: None,
            walrus_binary: default::walrus_binary(),
            walrus_config: None,
            walrus_package: None,
            gas_budget: default::gas_budget(),
        }
    }
}

impl GeneralArgs {
    /// Returns the wallet context from the configuration.
    ///
    /// If no wallet is specified, the default wallet will be used.
    pub fn load_wallet(&self) -> Result<WalletContext> {
        load_wallet_context(
            self.wallet.as_deref(),
            self.wallet_env.as_deref(),
            self.wallet_address.as_ref(),
        )
    }
}

macro_rules! merge {
    ($self:ident, $other:ident, $field:ident) => {
        let self_value = $self.$field.as_ref();
        let other_value = $other.$field.as_ref();
        let field = stringify!($field);

        if $other.$field.is_some() {
            tracing::debug!(?self_value, ?other_value, field, "merging field",);
            $self.$field = $other.$field.clone();
        } else {
            tracing::debug!(?self_value, ?other_value, field, "not merging field");
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
            wallet_env,
            wallet_address,
            walrus_context,
            walrus_binary,
            walrus_config,
            walrus_package,
            gas_budget,
        );
    }
}

#[derive(Debug, Clone)]
pub enum ObjectIdOrName {
    /// The object ID of the site.
    ObjectId(ObjectID),
    /// The name of the site.
    Name(String),
}

impl ObjectIdOrName {
    fn parse_sitemap_target(input: &str) -> Result<Self> {
        if let Ok(object_id) = ObjectID::from_str(input) {
            Ok(Self::ObjectId(object_id))
        } else {
            Ok(Self::Name(Self::normalize_name(input)))
        }
    }

    /// Returns the object ID of the site, resolving it from SuiNS if necessary.
    pub(crate) async fn resolve_object_id(
        &self,
        client: RetriableSuiClient,
        context: &str,
    ) -> Result<ObjectID> {
        match self {
            ObjectIdOrName::ObjectId(object) => Ok(*object),
            ObjectIdOrName::Name(domain) => {
                let suins = SuiNsClient::from_context(client, context).await?;
                let record = suins.resolve_name_record(domain).await?;
                let object_id = record.walrus_site_id();

                if let Some(object_id) = object_id {
                    println!(
                        "The SuiNS name {domain} points to the Walrus Site object: {object_id} (on {context})"
                    );
                    Ok(object_id)
                } else {
                    Err(anyhow!(
                        "the SuiNS name ({}) provided does not point to any object",
                        domain
                    ))
                }
            }
        }
    }

    /// Normalizes the suins name, changing it to the format `<name>.sui`.
    pub(crate) fn normalize_name(name: &str) -> String {
        if let Some(stripped) = name.strip_prefix('@') {
            format!("{stripped}.sui")
        } else {
            name.to_owned()
        }
    }
}

#[derive(Subcommand, Debug, Clone)]
#[command(rename_all = "kebab-case")]
pub enum Commands {
    /// Deploy a new site on Sui.
    ///
    /// If the site has not been published before, this command publishes it and stores
    /// the object ID of the Site in the ws-resources.json file.
    /// If the site has been published before, this command updates the site(indicaded
    /// by the site_object_id field in the ws-resources.json file).
    Deploy {
        #[clap(flatten)]
        publish_options: PublishOptions,
        /// The name of the site.
        #[arg(short, long)]
        site_name: Option<String>,
        /// The object ID of a partially published site to be completed.
        ///
        /// This is the object ID of the site that was published before, and is now being updated.
        /// If this is provided, it will override the object ID in the ws-resources.json file.
        #[arg(short, long)]
        object_id: Option<ObjectID>,
        /// Watch the site directory for changes and automatically redeploy when files are modified.
        ///
        /// When enabled, the command will continuously monitor the site directory and trigger a
        /// redepoloyment whenever changes are detected, allowing for rapid development iteration.
        #[arg(short, long)]
        watch: bool,
        /// Checks and extends all blobs in an existing site during an update.
        ///
        /// With this flag, the site-builder will force a check of the status of all the Walrus
        /// blobs composing the site, and will extend the ones that expire before `--epochs` epochs.
        /// This is useful to ensure all the resources in the site are available for the same
        /// amount of epochs.
        ///
        /// Further, when this flag is set, _missing_ blobs will also be reuploaded (e.g., in case
        /// they were deleted, or are all expired and were not owned, or, in case of testnet, the
        /// network was wiped).
        ///
        /// Without this flag, when updating a site, the `deploy` command will only create new blobs
        /// for the resources that have been added or modified (compared to the object on Sui).
        /// This implies that successive updates (without --check-extend) may result in the site
        /// having resources with different expiration times (and possibly some that are expired).
        #[arg(long)]
        check_extend: bool,
    },
    /// Publish a new site on Sui.
    Publish {
        #[clap(flatten)]
        publish_options: PublishOptions,
        /// The name of the site.
        #[arg(short, long)]
        site_name: Option<String>,
    },
    /// Update an existing site.
    Update {
        #[clap(flatten)]
        publish_options: PublishOptions,
        /// The object ID of a partially published site to be completed.
        object_id: ObjectID,
        /// Watch the site directory for changes and automatically redeploy when files are modified.
        ///
        /// When enabled, the command will continuously monitor the site directory and trigger a
        /// redepoloyment whenever changes are detected, allowing for rapid development iteration.
        #[arg(short, long)]
        watch: bool,
        /// This flag is deprecated and will be removed in the future. Use --check-extend.
        ///
        /// Publish all resources to Sui and Walrus, even if they may be already present.
        /// This can be useful in case the Walrus devnet is reset, but the resources are still
        /// available on Sui.
        #[arg(long)]
        #[deprecated(note = "This flag is being removed; please use --check-extend")]
        force: bool,
        /// Checks and extends all blobs in the site during the update.
        ///
        /// With this flag, the site-builder will force a check of the status of all the Walrus
        /// blobs composing the site, and will extend the ones that expire before `--epochs` epochs.
        /// This is useful to ensure all the resources in the site are available for the same
        /// amount of epochs.
        ///
        /// Further, when this flag is set, _missing_ blobs will also be reuploaded (e.g., in case
        /// they were deleted, or are all expired and were not owned, or, in case of testnet, the
        /// network was wiped).
        ///
        /// Without this flag, the `update` command will only create new blobs for the resources
        /// that have been added or modified (compared to the object on Sui). This implies that
        /// successive updates (without --check-extend) may result in the site having resources
        /// with different expiration times (and possibly some that are expired).
        #[arg(long)]
        check_extend: bool,
    },
    /// Convert an object ID in hex format to the equivalent Base36 format.
    ///
    /// This command may be useful to browse a site, given it object ID.
    Convert {
        /// The object id (in hex format) to convert
        object_id: ObjectID,
    },
    /// Show the pages composing the site at the given object ID or the given SuiNS name.
    ///
    /// Running this command requires the `walrus_package` to be specified either in the config or
    /// through the `--walrus-package` flag.
    Sitemap {
        #[arg(value_parser = ObjectIdOrName::parse_sitemap_target)]
        /// The site to be mapped.
        ///
        /// The site can be specified as object ID (in hex form) or as SuiNS name.
        /// The SuiNS name can be specified either as `<name>.sui`, or as `@<name>`,
        site_to_map: ObjectIdOrName,
    },
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
        #[arg(long)]
        resource: PathBuf,
        /// The path the resource should have in the site.
        ///
        /// Should be in the form `/path/to/resource.html`, with a leading `/`.
        #[arg(long)]
        path: String,
        /// The object ID of the Site object on Sui, to which the resource will be added.
        #[arg(long)]
        site_object: ObjectID,
        /// Common configurations.
        #[clap(flatten)]
        common: WalrusStoreOptions,
    },
}

#[derive(Parser, Debug, Clone)]
pub struct PublishOptions {
    /// The directory containing the site sources.
    pub directory: PathBuf,
    /// Preprocess the directory before publishing.
    /// See the `list-directory` command. Warning: Rewrites all `index.html` files.
    #[arg(long)]
    pub list_directory: bool,
    // TODO(nikos) deprecated note
    /// The maximum number of concurrent calls to the Walrus CLI for the computation of blob IDs.
    #[arg(long)]
    #[deprecated(note = "This flag is being removed")]
    pub max_concurrent: Option<NonZeroUsize>,
    /// The maximum number of blobs that can be stored concurrently.
    ///
    /// More blobs can be stored concurrently, but this will increase memory usage.
    #[arg(long, default_value_t = default::max_parallel_stores())]
    pub max_parallel_stores: NonZeroUsize,
    /// Common configurations.
    #[clap(flatten)]
    pub walrus_options: WalrusStoreOptions,
}

#[derive(Parser, Debug, Clone, Default)]
/// Common configurations across publish, update, and update-resource commands.
pub struct WalrusStoreOptions {
    /// The path to the Walrus sites resources file.
    ///
    /// This JSON configuration file defined HTTP resource headers and other utilities for your
    /// files. By default, the file is expected to be named `ws-resources.json` and located in the
    /// root of the site directory.
    ///
    /// The configuration file _will not_ be uploaded to Walrus.
    #[arg(long)]
    pub ws_resources: Option<PathBuf>,
    /// The epoch argument to specify either the number of epochs to store the blob, or the
    /// end epoch, or the earliest expiry time in rfc3339 format.
    ///
    #[command(flatten)]
    pub epoch_arg: EpochArg,
    /// Make the stored resources permanent.
    ///
    /// By default, sites are deletable with site-builder delete command. By passing --permanent,
    /// the site is deleted only after `epochs` expiration. Make resources permanent
    /// (non-deletable)
    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub permanent: bool,
    /// Perform a dry run (you'll be asked for confirmation before committing changes).
    #[arg(long)]
    pub dry_run: bool,
}

/// The number of epochs to store the blob for.
#[derive(Parser, Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[command(group(
    ArgGroup::new("epoch_arg")
        .args(&["epochs", "earliest_expiry_time", "end_epoch"])
        .required(true)
))]
#[serde(rename_all = "camelCase")]
pub struct EpochArg {
    /// The number of epochs the blob is stored for.
    ///
    /// If set to `max`, the blob is stored for the maximum number of epochs allowed by the
    /// system object on chain. Otherwise, the blob is stored for the specified number of
    /// epochs. The number of epochs must be greater than 0.
    #[arg(long, value_parser = EpochCountOrMax::parse_epoch_count)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epochs: Option<EpochCountOrMax>,

    /// The earliest time when the blob can expire, in RFC3339 format (e.g., "2024-03-20T15:00:00Z")
    /// or a more relaxed format (e.g., "2024-03-20 15:00:00").
    #[arg(long, value_parser = humantime::parse_rfc3339_weak)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub earliest_expiry_time: Option<SystemTime>,

    /// The end epoch for the blob.
    #[arg(long = "end-epoch")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_epoch: Option<NonZeroU32>,
}

/// The number of epochs to store the blobs for.
///
/// Can be either a non-zero number of epochs or the special value `max`, which will store the blobs
/// for the maximum number of epochs allowed by the system object on chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EpochCountOrMax {
    /// Store the blobs for the maximum number of epochs allowed.
    #[serde(rename = "max")]
    Max,
    /// The number of epochs to store the blobs for.
    #[serde(untagged)]
    Epochs(NonZeroU32),
}

impl Default for EpochCountOrMax {
    fn default() -> Self {
        Self::Epochs(NonZeroU32::new(1).unwrap())
    }
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

pub mod default {
    use std::num::NonZeroUsize;

    pub const DEFAULT_SITE_NAME: &str = "My Walrus Site";
    pub const DEFAULT_WS_RESOURCES_FILE: &str = "ws-resources.json";

    pub fn walrus_binary() -> Option<String> {
        Some("walrus".to_owned())
    }
    pub fn gas_budget() -> Option<u64> {
        Some(500_000_000)
    }
    pub fn max_parallel_stores() -> NonZeroUsize {
        NonZeroUsize::new(50).unwrap()
    }
}

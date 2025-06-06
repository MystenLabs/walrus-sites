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
    /// The address to be used for the Sui wallet.
    ///
    /// If not specified, the address specified in the sites-config (under `wallet_address`) will be
    /// used. If the wallet address is also not specified in the config, the address configured in
    /// the Sui client will be used.
    #[clap(long)]
    pub(crate) wallet_address: Option<SuiAddress>,
    /// The context that will be passed to the Walrus binary.
    ///
    /// If not specified, the Walrus context specified in the sites-config will be
    /// used. If it is also not specified in the config, no context will be passed.
    #[clap(long)]
    pub(crate) walrus_context: Option<String>,
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
    /// The package ID of the Walrus package on the selected network.
    ///
    /// This is currently only used for the `sitemap` command.
    #[clap(long)]
    pub(crate) walrus_package: Option<ObjectID>,
    /// The gas budget for the operations on Sui.
    ///
    /// Can be specified as a CLI argument or in the config.
    #[clap(long)]
    #[serde(default = "default::gas_budget")]
    pub(crate) gas_budget: Option<u64>,
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
            gas_budget,
        );
    }
}

#[derive(Debug, Clone)]
pub(crate) enum ObjectIdOrName {
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
                        "The SuiNS name {} points to the Walrus Site object: {} (on {})",
                        domain, object_id, context
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
            format!("{}.sui", stripped)
        } else {
            name.to_owned()
        }
    }
}

#[derive(Subcommand, Debug)]
#[clap(rename_all = "kebab-case")]
pub(crate) enum Commands {
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
        #[clap(short, long)]
        site_name: Option<String>,
        /// The object ID of a partially published site to be completed.
        ///
        /// This is the object ID of the site that was published before, and is now being updated.
        /// If this is provided, it will override the object ID in the ws-resources.json file.
        #[clap(short, long)]
        object_id: Option<ObjectID>,
        /// Watch the site directory for changes and automatically redeploy when files are modified.
        ///
        /// When enabled, the command will continuously monitor the site directory and trigger a
        /// redepoloyment whenever changes are detected, allowing for rapid development iteration.
        #[clap(short, long, action)]
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
        #[clap(long, action)]
        check_extend: bool,
    },
    /// Publish a new site on Sui.
    Publish {
        #[clap(flatten)]
        publish_options: PublishOptions,
        /// The name of the site.
        #[clap(short, long)]
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
        #[clap(short, long, action)]
        watch: bool,
        /// This flag is deprecated and will be removed in the future. Use --check-extend.
        ///
        /// Publish all resources to Sui and Walrus, even if they may be already present.
        /// This can be useful in case the Walrus devnet is reset, but the resources are still
        /// available on Sui.
        #[clap(long, action)]
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
        #[clap(long, action)]
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
        #[clap(value_parser = ObjectIdOrName::parse_sitemap_target)]
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
        /// Common configurations.
        #[clap(flatten)]
        common: WalrusStoreOptions,
    },
}

#[derive(Parser, Debug, Clone)]
pub(crate) struct PublishOptions {
    /// The directory containing the site sources.
    pub(crate) directory: PathBuf,
    /// Preprocess the directory before publishing.
    /// See the `list-directory` command. Warning: Rewrites all `index.html` files.
    #[clap(long, action)]
    pub(crate) list_directory: bool,
    /// The maximum number of concurrent calls to the Walrus CLI for the computation of blob IDs.
    #[clap(long)]
    pub(crate) max_concurrent: Option<NonZeroUsize>,
    /// The maximum number of blobs that can be stored concurrently.
    ///
    /// More blobs can be stored concurrently, but this will increase memory usage.
    #[clap(long, default_value_t = default::max_parallel_stores())]
    pub(crate) max_parallel_stores: NonZeroUsize,
    /// Common configurations.
    #[clap(flatten)]
    pub(crate) walrus_options: WalrusStoreOptions,
}

#[derive(Parser, Debug, Clone, Default)]
/// Common configurations across publish, update, and update-resource commands.
pub(crate) struct WalrusStoreOptions {
    /// The path to the Walrus sites resources file.
    ///
    /// This JSON configuration file defined HTTP resource headers and other utilities for your
    /// files. By default, the file is expected to be named `ws-resources.json` and located in the
    /// root of the site directory.
    ///
    /// The configuration file _will not_ be uploaded to Walrus.
    #[clap(long)]
    pub(crate) ws_resources: Option<PathBuf>,
    /// The epoch argument to specify either the number of epochs to store the blob, or the
    /// end epoch, or the earliest expiry time in rfc3339 format.
    ///
    #[command(flatten)]
    pub(crate) epoch_arg: EpochArg,
    // pub(crate) epochs: EpochCountOrMax,
    /// Make the stored resources permanent.
    ///
    /// By default, sites are deletable with site-builder delete command. By passing --permanent,
    /// the site is deleted only after `epochs` expiration. Make resources permanent
    /// (non-deletable)
    #[clap(long, action = clap::ArgAction::SetTrue)]
    pub(crate) permanent: bool,
    /// Perform a dry run (you'll be asked for confirmation before committing changes).
    #[clap(long)]
    pub(crate) dry_run: bool,
}

/// The number of epochs to store the blob for.
#[derive(Parser, Debug, Clone, Default, Serialize, Deserialize)]
#[clap(group(
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
    #[clap(long = "end-epoch")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_epoch: Option<NonZeroU32>,
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

mod default {
    use std::num::NonZeroUsize;

    pub(crate) fn walrus_binary() -> Option<String> {
        Some("walrus".to_owned())
    }
    pub(crate) fn gas_budget() -> Option<u64> {
        Some(500_000_000)
    }
    pub(crate) fn max_parallel_stores() -> NonZeroUsize {
        NonZeroUsize::new(50).unwrap()
    }
}

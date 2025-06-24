// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{num::NonZeroU32, path::PathBuf, time::SystemTime};

use site_builder::args::{EpochArg, EpochCountOrMax, WalrusStoreOptions};
use thiserror::Error;

/// Common configurations across publish, update, and update-resource commands.
#[derive(Debug, Clone, Default)]
pub struct WalrusStoreOptionsBuilder {
    /// The path to the Walrus sites resources file.
    ///
    /// This JSON configuration file defined HTTP resource headers and other utilities for your
    /// files. By default, the file is expected to be named `ws-resources.json` and located in the
    /// root of the site directory.
    ///
    /// The configuration file _will not_ be uploaded to Walrus.
    ws_resources: Option<PathBuf>,
    /// The epoch argument to specify either the number of epochs to store the blob, or the
    /// end epoch, or the earliest expiry time in rfc3339 format.
    ///
    epoch_arg: Option<EpochArgEnum>,
    /// Make the stored resources permanent.
    ///
    /// By default, sites are deletable with site-builder delete command. By passing --permanent,
    /// the site is deleted only after `epochs` expiration. Make resources permanent
    /// (non-deletable)
    permanent: bool,
    /// Perform a dry run (you'll be asked for confirmation before committing changes).
    dry_run: bool,
}

#[derive(Debug, Clone)]
enum EpochArgEnum {
    /// The number of epochs the blob is stored for.
    ///
    /// If set to `max`, the blob is stored for the maximum number of epochs allowed by the
    /// system object on chain. Otherwise, the blob is stored for the specified number of
    /// epochs. The number of epochs must be greater than 0.
    EpochCount(EpochCountOrMax),
    /// The earliest time when the blob can expire, in RFC3339 format (e.g., "2024-03-20T15:00:00Z")
    /// or a more relaxed format (e.g., "2024-03-20 15:00:00").
    EarliestExpiryTime(SystemTime),
    /// The end epoch for the blob.
    EndEpoch(NonZeroU32),
}

#[derive(Debug, Error)]
pub enum InvalidWalrusStoreOptionsConfig {
    #[error("PublishOptions needs epoch_arg. Try using `.with_epoch_count_or_max(epoch_arg_enum)` or `.with_earliest_expiry_time` or `.with_end_epoch`.")]
    MissingEpochs,
}

impl WalrusStoreOptionsBuilder {
    pub fn build(self) -> Result<WalrusStoreOptions, InvalidWalrusStoreOptionsConfig> {
        let WalrusStoreOptionsBuilder {
            ws_resources,
            epoch_arg,
            permanent,
            dry_run,
        } = self;
        let Some(epoch_arg) = epoch_arg else {
            return Err(InvalidWalrusStoreOptionsConfig::MissingEpochs);
        };

        let epoch_arg = match epoch_arg {
            EpochArgEnum::EpochCount(epochs) => EpochArg {
                epochs: Some(epochs),
                ..Default::default()
            },
            EpochArgEnum::EarliestExpiryTime(expiry_time) => EpochArg {
                earliest_expiry_time: Some(expiry_time),
                ..Default::default()
            },
            EpochArgEnum::EndEpoch(epoch) => EpochArg {
                end_epoch: Some(epoch),
                ..Default::default()
            },
        };

        Ok(WalrusStoreOptions {
            ws_resources,
            epoch_arg,
            permanent,
            dry_run,
        })
    }

    pub fn with_ws_resources(mut self, ws_resources: Option<PathBuf>) -> Self {
        self.ws_resources = ws_resources;
        self
    }

    pub fn with_epoch_count_or_max(mut self, epoch_count_or_max: EpochCountOrMax) -> Self {
        self.epoch_arg = Some(EpochArgEnum::EpochCount(epoch_count_or_max));
        self
    }

    pub fn with_earliest_expiry_time(mut self, earliest_expiry_time: SystemTime) -> Self {
        self.epoch_arg = Some(EpochArgEnum::EarliestExpiryTime(earliest_expiry_time));
        self
    }

    pub fn with_end_epoch(mut self, end_epoch: NonZeroU32) -> Self {
        self.epoch_arg = Some(EpochArgEnum::EndEpoch(end_epoch));
        self
    }

    pub fn with_permanent(mut self, permanent: bool) -> Self {
        self.permanent = permanent;
        self
    }

    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }
}

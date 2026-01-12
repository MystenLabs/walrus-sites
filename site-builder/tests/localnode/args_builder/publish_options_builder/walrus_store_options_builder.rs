// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{num::NonZeroU32, path::PathBuf, time::SystemTime};

use bytesize::ByteSize;
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
    epoch_arg: Option<EpochArg>,
    /// Make the stored resources permanent.
    ///
    /// By default, sites are deletable with site-builder delete command. By passing --permanent,
    /// the site is deleted only after `epochs` expiration. Make resources permanent
    /// (non-deletable)
    permanent: bool,
    /// Perform a dry run (you'll be asked for confirmation before committing changes).
    dry_run: bool,
    /// Limits the max total size of all the files stored per Quilt.
    ///
    /// Supports both decimal (KB, MB, GB) and binary (KiB, MiB, GiB) units, or plain byte numbers.
    /// Examples: "512MiB", "1GB", "1048576".
    max_quilt_size: Option<ByteSize>,
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
            max_quilt_size,
        } = self;
        let Some(epoch_arg) = epoch_arg else {
            return Err(InvalidWalrusStoreOptionsConfig::MissingEpochs);
        };

        Ok(WalrusStoreOptions {
            ws_resources,
            epoch_arg,
            permanent,
            dry_run,
            max_quilt_size: max_quilt_size.unwrap_or(ByteSize::mib(512)),
        })
    }

    pub fn with_ws_resources(mut self, ws_resources: Option<PathBuf>) -> Self {
        self.ws_resources = ws_resources;
        self
    }

    pub fn with_epoch_count_or_max(mut self, epoch_count_or_max: EpochCountOrMax) -> Self {
        self.epoch_arg = Some(EpochArg {
            epochs: Some(epoch_count_or_max),
            ..Default::default()
        });
        self
    }

    pub fn with_earliest_expiry_time(mut self, earliest_expiry_time: SystemTime) -> Self {
        self.epoch_arg = Some(EpochArg {
            earliest_expiry_time: Some(earliest_expiry_time),
            ..Default::default()
        });
        self
    }

    pub fn with_end_epoch(mut self, end_epoch: NonZeroU32) -> Self {
        self.epoch_arg = Some(EpochArg {
            end_epoch: Some(end_epoch),
            ..Default::default()
        });
        self
    }

    pub fn with_epoch_arg(mut self, epoch_arg: EpochArg) -> Self {
        self.epoch_arg = Some(epoch_arg);
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

    pub fn with_max_quilt_size(mut self, max_quilt_size: ByteSize) -> Self {
        self.max_quilt_size = Some(max_quilt_size);
        self
    }
}

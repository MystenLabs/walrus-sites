// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{num::NonZeroU32, path::PathBuf, time::SystemTime};

use site_builder::args::{EpochArg, EpochCountOrMax, PublishOptions};
use thiserror::Error;

pub mod walrus_store_options_builder;
use walrus_store_options_builder::{InvalidWalrusStoreOptionsConfig, WalrusStoreOptionsBuilder};

#[derive(Debug, Clone, Default)]
pub struct PublishOptionsBuilder {
    /// The directory containing the site sources.
    pub directory: Option<PathBuf>,
    /// Preprocess the directory before publishing.
    /// See the `list-directory` command. Warning: Rewrites all `index.html` files.
    pub list_directory: bool,
    /// Common configurations.
    // Note: We are currently re-using `WalrusStoreOptionsBuilder`'s methods for convenience,
    // and keeping walrus_store_options_builder mod private.
    pub walrus_options: WalrusStoreOptionsBuilder,
}

#[derive(Debug, Error)]
pub enum InvalidPublishOptionsConfig {
    #[error("PublishOptions need a directory. Try using `.with_directory(path_buf)`.")]
    MissingDirectory,
    #[error(transparent)]
    MissingEpochs(#[from] InvalidWalrusStoreOptionsConfig),
}

impl PublishOptionsBuilder {
    pub fn build(self) -> Result<PublishOptions, InvalidPublishOptionsConfig> {
        let PublishOptionsBuilder {
            directory,
            list_directory,
            walrus_options,
        } = self;
        let Some(directory) = directory else {
            return Err(InvalidPublishOptionsConfig::MissingDirectory);
        };

        let walrus_options = walrus_options.build()?;

        Ok(PublishOptions {
            directory,
            list_directory,
            walrus_options,
        })
    }

    pub fn with_directory(mut self, directory: PathBuf) -> Self {
        self.directory.replace(directory);
        self
    }

    pub fn with_list_directory(mut self, list_directory: bool) -> Self {
        self.list_directory = list_directory;
        self
    }

    pub fn with_ws_resources(mut self, ws_resources: Option<PathBuf>) -> Self {
        self.walrus_options = self.walrus_options.with_ws_resources(ws_resources);
        self
    }

    pub fn with_epoch_count_or_max(mut self, epoch_count_or_max: EpochCountOrMax) -> Self {
        self.walrus_options = self
            .walrus_options
            .with_epoch_count_or_max(epoch_count_or_max);
        self
    }

    pub fn with_earliest_expiry_time(mut self, earliest_expiry_time: SystemTime) -> Self {
        self.walrus_options = self
            .walrus_options
            .with_earliest_expiry_time(earliest_expiry_time);
        self
    }

    pub fn with_end_epoch(mut self, end_epoch: NonZeroU32) -> Self {
        self.walrus_options = self.walrus_options.with_end_epoch(end_epoch);
        self
    }

    pub fn with_epoch_arg(mut self, epoch_arg: EpochArg) -> Self {
        self.walrus_options = self.walrus_options.with_epoch_arg(epoch_arg);
        self
    }

    pub fn with_permanent(mut self, permanent: bool) -> Self {
        self.walrus_options = self.walrus_options.with_permanent(permanent);
        self
    }

    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.walrus_options = self.walrus_options.with_dry_run(dry_run);
        self
    }

    pub fn with_max_quilt_size(mut self, max_quilt_size: bytesize::ByteSize) -> Self {
        self.walrus_options = self.walrus_options.with_max_quilt_size(max_quilt_size);
        self
    }
}

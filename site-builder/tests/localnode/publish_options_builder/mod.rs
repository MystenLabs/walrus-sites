use std::{
    num::{NonZeroU32, NonZeroUsize},
    path::PathBuf,
    time::SystemTime,
};

use site_builder::args::{default as sites_default, EpochCountOrMax, PublishOptions};
use thiserror::Error;

#[allow(dead_code)]
mod walrus_store_options_builder;
use walrus_store_options_builder::{InvalidWalrusStoreOptionsConfig, WalrusStoreOptionsBuilder};

#[derive(Debug, Clone)]
pub struct PublishOptionsBuilder {
    /// The directory containing the site sources.
    pub directory: Option<PathBuf>,
    /// Preprocess the directory before publishing.
    /// See the `list-directory` command. Warning: Rewrites all `index.html` files.
    pub list_directory: bool,
    /// The maximum number of concurrent calls to the Walrus CLI for the computation of blob IDs.
    pub max_concurrent: Option<NonZeroUsize>,
    /// The maximum number of blobs that can be stored concurrently.
    ///
    /// More blobs can be stored concurrently, but this will increase memory usage.
    // #[arg(long, default_value_t = default::max_parallel_stores())]
    pub max_parallel_stores: NonZeroUsize,
    /// Common configurations.
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
            max_concurrent,
            max_parallel_stores,
            walrus_options,
        } = self;
        let Some(directory) = directory else {
            return Err(InvalidPublishOptionsConfig::MissingDirectory);
        };

        let walrus_options = walrus_options.build()?;

        Ok(PublishOptions {
            directory,
            list_directory,
            max_concurrent,
            max_parallel_stores,
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

    pub fn with_max_concurrent(mut self, max_concurrent: Option<NonZeroUsize>) -> Self {
        self.max_concurrent = max_concurrent;
        self
    }

    pub fn with_max_parallel_stores(mut self, max_parallel_stores: NonZeroUsize) -> Self {
        self.max_parallel_stores = max_parallel_stores;
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

    pub fn with_permanent(mut self, permanent: bool) -> Self {
        self.walrus_options = self.walrus_options.with_permanent(permanent);
        self
    }

    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.walrus_options = self.walrus_options.with_dry_run(dry_run);
        self
    }
}

impl Default for PublishOptionsBuilder {
    fn default() -> Self {
        Self {
            directory: None,
            list_directory: false,
            max_concurrent: None,
            max_parallel_stores: sites_default::max_parallel_stores(),
            walrus_options: WalrusStoreOptionsBuilder::default(),
        }
    }
}

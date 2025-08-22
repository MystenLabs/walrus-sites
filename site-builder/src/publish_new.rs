// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;

use crate::{
    args::{default, PublishOptions},
    display,
    publish::load_ws_resources,
    site::config::WSResources,
    site_new::local_resource::manager::ResourceManager,
};

/// Separates the argument parsing from actually building the site.
#[derive(Debug)]
pub struct SitePublisherBuilder {
    pub context: Option<String>,
    pub site_name: Option<String>,
    // pub config: Config,
    pub publish_options: PublishOptions,
}

impl SitePublisherBuilder {
    // pub fn with_context(mut self, context: Option<String>) -> Self {
    //     self.context = context;
    //     self
    // }
    //
    // pub fn with_site_name(mut self, site_name: String) -> Self {
    //     self.site_name = Some(site_name);
    //     self
    // }
    //
    // pub fn with_publish_options(mut self, publish_options: PublishOptions) -> Self {
    //     self.publish_options = publish_options;
    //     self
    // }

    pub async fn build(self) -> anyhow::Result<SitePublisher> {
        let Self {
            context,
            site_name,
            publish_options,
        } = self;
        let PublishOptions {
            directory,
            list_directory: _, // TODO(nikos) handle list-directory
            max_concurrent,
            max_parallel_stores: _, // TODO(nikos) will proly need this later
            walrus_options,
        } = publish_options;
        let (ws_resources, ws_resources_path) =
            load_ws_resources(walrus_options.ws_resources.as_deref(), directory.as_path())?;

        let WSResources {
            headers,
            routes: _,   // TODO(nikos) will proly need this later
            metadata: _, // TODO(nikos) will proly need this later
            site_name: ws_site_name,
            object_id: _, // TODO(nikos) will proly need this later
            ignore,
        } = ws_resources.unwrap_or_default();
        let site_name = site_name
            .or(ws_site_name)
            .unwrap_or(default::DEFAULT_SITE_NAME.to_string());

        let resource_manager = ResourceManager::new(
            headers.unwrap_or_default(),
            ignore.unwrap_or_default(),
            ws_resources_path,
            max_concurrent,
        );

        Ok(SitePublisher {
            context,
            site_name,
            resource_manager,
            directory,
        })
    }
}

// TODO(nikos): Handle list-directory. To me it makes sense to be a separate command.
// Also I think it will be deprecated after File-manager in walrus is implemented.
pub struct SitePublisher {
    pub context: Option<String>,
    pub site_name: String,
    // TODO(nikos): We probably need to keep the path of the ws-resources in order to not upload.
    pub resource_manager: ResourceManager,
    // TODO(nikos): Does it make sense to include directory inside the new `ResourceManager` above?
    pub directory: PathBuf,
}

impl SitePublisher {
    pub async fn run(self) -> anyhow::Result<()> {
        let Self {
            context: _,   // TODO(nikos) will proly need this later
            site_name: _, // TODO(nikos) will proly need this later
            mut resource_manager,
            directory,
        } = self;

        display::action(format!(
            "Parsing the directory {}",
            directory.to_string_lossy()
        ));
        let resources = resource_manager.read_dir(directory.as_path()).await?;
        display::done();
        tracing::debug!(?resources, "resources loaded from directory");

        Ok(())
    }
}

// Gets the configuration from the provided file, or looks in the default directory.
/*
    async fn run_single_edit(
        &self,
    ) -> Result<(SuiAddress, SuiTransactionBlockResponse, SiteDataDiffSummary)> {
        if self.edit_options.publish_options.list_directory {
            display::action(format!("Preprocessing: {}", self.directory().display()));
            Preprocessor::preprocess(self.directory())?;
            display::done();
        }

        // Note: `load_ws_resources` again. We already loaded them when parsing the name.
        let (ws_resources, ws_resources_path) = load_ws_resources(
            self.edit_options
                .publish_options
                .walrus_options
                .ws_resources
                .as_deref(),
            self.directory(),
        )?;
        if let Some(path) = ws_resources_path.as_ref() {
            println!(
                "Using the Walrus sites resources file: {}",
                path.to_string_lossy()
            );
        }

        let mut resource_manager = ResourceManager::new(
            self.config.walrus_client(),
            ws_resources.clone(),
            ws_resources_path.clone(),
            self.edit_options.publish_options.max_concurrent,
        )
        .await?;
        display::action(format!(
            "Parsing the directory {} and locally computing blob IDs",
            self.directory().to_string_lossy()
        ));
        let local_site_data = resource_manager.read_dir(self.directory()).await?;
        display::done();
        tracing::debug!(?local_site_data, "resources loaded from directory");

        let site_metadata = match ws_resources.clone() {
            Some(value) => value.metadata,
            None => None,
        };

        let site_name = ws_resources.as_ref().and_then(|r| r.site_name.clone());

        let mut site_manager = SiteManager::new(
            self.config.clone(),
            self.edit_options.site_id,
            self.edit_options.blob_options.clone(),
            self.edit_options.publish_options.walrus_options.clone(),
            site_metadata,
            self.edit_options.site_name.clone().or(site_name),
            self.edit_options.publish_options.max_parallel_stores,
        )
        .await?;

        let (response, summary) = site_manager.update_site(&local_site_data).await?;

        let path_for_saving =
            ws_resources_path.unwrap_or_else(|| self.directory().join(DEFAULT_WS_RESOURCES_FILE));

        persist_site_identifier(
            &self.edit_options.site_id,
            &site_manager,
            &response,
            ws_resources,
            &path_for_saving,
        )?;

        Ok((site_manager.active_address()?, response, summary))
    }
*/

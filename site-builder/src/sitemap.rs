// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Utilities to create the site map of a site.

use std::collections::HashMap;

use anyhow::{anyhow, bail, Context, Result};
use prettytable::{
    format::{self, FormatBuilder},
    row,
    Cell,
    Table,
};
use sui_types::base_types::{ObjectID, SuiAddress};

use crate::{
    args::ObjectIdOrName,
    backoff::ExponentialBackoffConfig,
    config::Config,
    retry_client::RetriableSuiClient,
    site::{RemoteSiteFactory, SiteData},
    util::type_origin_map_for_package,
    walrus::{output::SuiBlob, types::BlobId},
};

/// Displays the map of a site.
///
/// This contains the resources in the site, their blob IDs, and the IDs of owned Blob objects that
/// have that blob ID.
pub(crate) async fn display_sitemap(
    site_to_map: ObjectIdOrName,
    selected_context: Option<String>,
    config: Config,
) -> Result<()> {
    let Some(context) = selected_context else {
        bail!(
            "the sitemap command requires a context to be specified, either in the \
                    multi-config file or through the --context flag; supported options are \
                    'testnet' and 'mainnet'"
        );
    };
    let mut wallet = config.load_wallet()?;
    let owner_address = wallet.active_address()?;
    let sui_client =
        RetriableSuiClient::new_from_wallet(&wallet, ExponentialBackoffConfig::default()).await?;

    let site_object_id = site_to_map
        .resolve_object_id(sui_client.clone(), &context)
        .await
        .context("could not resolve the name to an object ID")?;
    let (site_data, owned_blobs) =
        get_site_resources_and_blobs(&sui_client, &config, owner_address, site_object_id).await?;

    if site_data.resources().is_empty() {
        println!("No resources found for the provided object ID or SuiNS name");
        return Ok(());
    }

    let table = MapTable::new(&site_data, &owned_blobs);
    table.printstd();

    Ok(())
}

async fn get_site_resources_and_blobs(
    sui_client: &RetriableSuiClient,
    config: &Config,
    owner_address: SuiAddress,
    site_object_id: ObjectID,
) -> Result<(SiteData, HashMap<BlobId, SuiBlob>)> {
    // Get all the blobs owned by the owner.
    let type_map = type_origin_map_for_package(
        sui_client,
        config.general.walrus_package.ok_or(anyhow!(
            "no walrus package specified; please add it to the config or \
            pass it with `--walrus-package`"
        ))?,
    )
    .await?;

    let owned_blobs = sui_client
        .get_owned_objects_of_type::<SuiBlob>(owner_address, &type_map, &[])
        .await?
        .map(|blob| (blob.blob_id, blob))
        .collect::<HashMap<_, _>>();

    let site = RemoteSiteFactory::new(sui_client, config.package)
        .await?
        .get_from_chain(site_object_id)
        .await?;

    Ok((site, owned_blobs))
}

struct MapTable(Vec<(String, BlobId, Option<ObjectID>)>);

impl MapTable {
    fn new(site_data: &SiteData, owned_blobs: &HashMap<BlobId, SuiBlob>) -> Self {
        let mut data = Vec::with_capacity(site_data.resources().len());
        site_data.resources().iter().for_each(|resource| {
            let info = &resource.info;
            let blob_object_id = owned_blobs.get(&info.blob_id).map(|blob| blob.id);
            data.push((info.path.clone(), info.blob_id, blob_object_id));
        });
        Self(data)
    }

    fn printstd(&self) {
        let mut table = Table::new();
        table.set_format(
            FormatBuilder::new()
                .separators(
                    &[
                        format::LinePosition::Top,
                        format::LinePosition::Bottom,
                        format::LinePosition::Title,
                    ],
                    format::LineSeparator::new('-', '-', '-', '-'),
                )
                .padding(1, 1)
                .build(),
        );

        let mut titles = row![
            b->"Resource path",
            b->"Blob ID",
        ];
        let has_blob_id = self
            .0
            .iter()
            .any(|(_, _, owned_blob_id)| owned_blob_id.is_some());

        if has_blob_id {
            titles.add_cell(Cell::new("Owned blob object ID (if any)").style_spec("b"));
        }
        table.set_titles(titles);

        for (owned_blob_id, blob_id, object_id) in &self.0 {
            let mut row = row![Cell::new(owned_blob_id), Cell::new(&blob_id.to_string())];

            if has_blob_id {
                row.add_cell(Cell::new(
                    &object_id.map_or_else(|| "".to_owned(), |id| id.to_string()),
                ));
            }
            table.add_row(row);
        }

        table.printstd();
    }
}

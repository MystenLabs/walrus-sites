// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Utilities to create the site map of a site.

use std::collections::HashMap;

use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Duration, NaiveDate};
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
    types::Staking,
    util::{get_owned_blobs, parse_quilt_patch_id},
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

    let staking_object_id = config
        .staking_object
        .ok_or_else(|| anyhow!("staking_object not defined in the config"))?;
    dbg!("Using staking object:", staking_object_id);

    let staking_object = sui_client
        .get_staking_object(staking_object_id)
        .await
        .context(format!(
            "Could not fetch staking object: {staking_object_id}"
        ))?;
    let table = SiteMapTable::new(&site_data, &owned_blobs, &staking_object);
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
    let walrus_package = config.general.resolve_walrus_package(sui_client).await?;
    let owned_blobs = get_owned_blobs(sui_client, walrus_package, owner_address)
        .await
        .context(format!(
            "Could not fetch owned blobs for address: {owner_address}"
        ))?
        .into_iter()
        .map(|(blob_id, (sui_blob, _obj_ref))| (blob_id, sui_blob))
        .collect();

    let site = RemoteSiteFactory::new(sui_client, config.package)
        .await?
        .get_from_chain(site_object_id)
        .await?;

    Ok((site, owned_blobs))
}

struct SiteMapTable(Vec<(String, String, Option<ObjectID>, Option<NaiveDate>)>);

impl SiteMapTable {
    fn new(
        site_data: &SiteData,
        owned_blobs: &HashMap<BlobId, SuiBlob>,
        staking_obj: &Staking,
    ) -> Self {
        let mut data = Vec::with_capacity(site_data.resources().len());

        let epoch_duration = staking_obj.inner.epoch_duration;
        let epoch_1_start =
            DateTime::from_timestamp_millis(staking_obj.inner.first_epoch_start as i64).unwrap();

        site_data.resources().iter().for_each(|resource| {
            let info = &resource.info;
            let blob_object_id = owned_blobs.get(&info.blob_id).map(|blob| blob.id);

            let expiration = owned_blobs.get(&info.blob_id).and_then(|blob| {
                let end_epoch = blob.storage.end_epoch as u64;
                let epoch_offset = end_epoch.saturating_sub(1);
                let total_ms = epoch_offset.checked_mul(epoch_duration)?;

                let secs = total_ms / 1000;
                let nanos = ((total_ms % 1000) * 1_000_000) as u32;

                Some(
                    (epoch_1_start
                        + Duration::seconds(secs as i64)
                        + Duration::nanoseconds(nanos as i64))
                    .date_naive(),
                )
            });

            if let Some(quilt_patch_id) = parse_quilt_patch_id(&info.blob_id, &info.headers) {
                data.push((
                    info.path.clone(),
                    quilt_patch_id.to_string(),
                    blob_object_id,
                    expiration,
                ));
            } else {
                data.push((
                    info.path.clone(),
                    info.blob_id.to_string(),
                    blob_object_id,
                    expiration,
                ));
            }
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
            b->"Blob / Quilt Patch ID",
        ];
        let has_blob_id = self
            .0
            .iter()
            .any(|(_, _, owned_blob_id, _)| owned_blob_id.is_some());
        let has_expiration = self
            .0
            .iter()
            .any(|(_, _, _, expiration)| expiration.is_some());

        if has_blob_id {
            titles.add_cell(Cell::new("Owned blob object ID (if any)").style_spec("b"));
        }
        if has_expiration {
            titles.add_cell(Cell::new("Earliest Expiration Date").style_spec("b"));
        }

        table.set_titles(titles);

        for (owned_blob_id, blob_id, object_id, expiration_opt) in &self.0 {
            let mut row = row![Cell::new(owned_blob_id), Cell::new(&blob_id.to_string())];

            if has_blob_id {
                row.add_cell(Cell::new(
                    &object_id.map_or_else(|| "".to_owned(), |id| id.to_string()),
                ));
            }
            if has_expiration {
                let exp_str = expiration_opt
                    .map(|dt| dt.format("%Y-%m-%d").to_string())
                    .unwrap_or_else(|| "-".to_string());
                row.add_cell(Cell::new(&exp_str));
            }

            table.add_row(row);
        }

        table.printstd();
    }
}

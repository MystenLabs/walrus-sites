// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Utilities to create the site map of a site.

use std::{collections::HashMap, fs};

use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Duration, NaiveDate};
use prettytable::{
    format::{self, FormatBuilder},
    row,
    Cell,
    Table,
};
use serde::Deserialize;
use sui_sdk::rpc_types::SuiObjectDataOptions;
use sui_types::base_types::{ObjectID, ObjectType, SuiAddress};

use crate::{
    args::ObjectIdOrName,
    backoff::ExponentialBackoffConfig,
    config::Config,
    retry_client::RetriableSuiClient,
    site::{RemoteSiteFactory, SiteData},
    types::Staking,
    util::{get_staking_object, parse_quilt_patch_id, type_origin_map_for_package},
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

    let staking_object = get_staking_object(&sui_client, staking_object_id).await?;
    let table = SiteMapTable::new(&site_data, &owned_blobs, &staking_object);
    table.printstd();

    Ok(())
}

// TODO(sew-495): Move it move it
// #[serde_as]
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
/// Configuration for the contract packages and shared objects.
pub struct WalrusContractConfig {
    /// Object ID of the Walrus system object.
    pub system_object: ObjectID,
    /// Object ID of the Walrus staking object.
    pub staking_object: ObjectID,
    /// Object ID of the credits object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credits_object: Option<ObjectID>,
    /// Object ID of the walrus subsidies object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub walrus_subsidies_object: Option<ObjectID>,
    // /// The TTL for cached system and staking objects.
    // #[serde(default = "defaults::default_cache_ttl", rename = "cache_ttl_secs")]
    // #[serde_as(as = "DurationSeconds")]
    // pub cache_ttl: Duration,
}

// TODO(sew-495): Move it somewhere else
pub async fn get_owned_blobs(
    sui_client: &RetriableSuiClient,
    config: &Config,
    owner_address: SuiAddress,
) -> anyhow::Result<HashMap<BlobId, SuiBlob>> {
    let walrus_package = match config.general.walrus_package {
        Some(pkg) => pkg,
        None => {
            // TODO: Maybe we want to parse from the walrus-config everywhere, not just for site-map and
            // when this is called.
            let walrus_config_path = config.general.walrus_config.as_ref().ok_or_else(|| {
                anyhow!("no walrus package, or walrus config specified; please add either")
            })?; // TODO(sew-495)
            let config_contents = fs::read_to_string(walrus_config_path)
                .context("Failed to read walrus config file")?;
            let walrus_contract_config: WalrusContractConfig =
                serde_yaml::from_str(&config_contents)
                    .context("Failed to parse walrus config file")?;

            let staking_obj = sui_client
                .get_object_with_options(
                    walrus_contract_config.staking_object,
                    SuiObjectDataOptions::new().with_type(),
                )
                .await
                .context("Failed to fetch staking object data")?;
            let ObjectType::Struct(move_object_type) = staking_obj
                .data
                .ok_or(anyhow!(
                    "Expected data in get-object response for staking-object"
                ))?
                .object_type()?
            else {
                bail!("Staking object ID points to a package") // TODO(sew-495): Improve
            };
            ObjectID::from_address(move_object_type.address())
        }
    };

    let type_map = type_origin_map_for_package(sui_client, walrus_package).await?;
    let blobs = sui_client
        .get_owned_objects_of_type::<SuiBlob>(owner_address, &type_map, &[])
        .await?
        .map(|blob| (blob.blob_id, blob))
        .collect();
    Ok(blobs)
}

async fn get_site_resources_and_blobs(
    sui_client: &RetriableSuiClient,
    config: &Config,
    owner_address: SuiAddress,
    site_object_id: ObjectID,
) -> Result<(SiteData, HashMap<BlobId, SuiBlob>)> {
    // Get all the blobs owned by the owner.
    let owned_blobs = get_owned_blobs(sui_client, config, owner_address).await?;

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

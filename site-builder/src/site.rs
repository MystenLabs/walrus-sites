// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

pub mod builder;
pub mod config;
pub mod content;
pub mod contracts;
pub mod estimates;
pub mod manager;
pub mod quilts;
pub mod resource;

use std::{collections::HashMap, time::Duration};

use anyhow::{Context, Result};
use futures::future::try_join_all;
use resource::{ResourceSet, SiteOps};
use sui_sdk::rpc_types::{DynamicFieldInfo, SuiObjectDataOptions};
use sui_types::{base_types::ObjectID, dynamic_field::derive_dynamic_field_id, TypeTag};
use walrus_sui::contracts::TypeOriginMap;

use crate::{
    retry_client::RetriableSuiClient,
    summary::SiteDataDiffSummary,
    types::{
        ExtendOps,
        Metadata,
        MetadataOp,
        RedirectOps,
        Redirects,
        ResourceDynamicField,
        RouteOps,
        Routes,
        SiteFields,
        SiteNameOp,
        SuiDynamicField,
    },
    util::handle_pagination,
};

pub const SITE_MODULE: &str = "site";

/// The dynamic field key for routes (matches `ROUTES_FIELD` in site.move).
const ROUTES_DF_KEY: &[u8] = b"routes";
/// The dynamic field key for redirects (matches `redirects_field!()` in redirects.move).
const REDIRECTS_DF_KEY: &[u8] = b"redirects";

/// The maximum number of dynamic fields to request at once.
const DF_REQ_BATCH_SIZE: usize = 10;
/// The delay between requests for dynamic fields.
const DF_REQ_DELAY_MS: u64 = 100;

/// The diff between two site data.
#[derive(Debug, Clone)]
pub struct SiteDataDiff<'a> {
    /// The operations to perform on the resources.
    pub resource_ops: Vec<SiteOps<'a>>,
    pub route_ops: RouteOps,
    pub redirect_ops: RedirectOps,
    pub metadata_op: MetadataOp,
    pub site_name_op: SiteNameOp,
    pub extend_ops: ExtendOps,
}

impl SiteDataDiff<'_> {
    /// Returns `true` if there are updates to be made.
    pub fn has_updates(&self) -> bool {
        self.resource_ops.iter().any(|op| op.is_resource_change())
            || !self.route_ops.is_unchanged()
            || !self.redirect_ops.is_unchanged()
            || !self.metadata_op.is_noop()
            || !self.site_name_op.is_noop()
            || !self.extend_ops.is_noop()
    }

    /// Returns the summary of the operations in the diff.
    pub fn summary(&self) -> SiteDataDiffSummary {
        SiteDataDiffSummary {
            resource_ops: self
                .resource_ops
                .iter()
                .filter(|op| op.is_resource_change())
                .map(|op| op.into())
                .collect(),
            route_ops: self.route_ops.clone(),
            redirect_ops: self.redirect_ops.clone(),
            metadata_updated: !self.metadata_op.is_noop(),
            site_name_updated: !self.site_name_op.is_noop(),
            extend_ops: self.extend_ops.clone(),
        }
    }
}

// TODO(sew-166): Blobs make sense to exist as a field instead of just fetched. This struct was made
// to compare sites. Now we also do some basic blob-lifetime-management.
// It might make more sense to track the blob-id here as well?
/// The site on chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteData {
    resources: ResourceSet,
    routes: Option<Routes>,
    redirects: Option<Redirects>,
    metadata: Option<Metadata>,
    site_name: Option<String>,
}

impl SiteData {
    /// SiteData constructor.
    pub fn new(
        resources: ResourceSet,
        routes: Option<Routes>,
        redirects: Option<Redirects>,
        metadata: Option<Metadata>,
        site_name: Option<String>,
    ) -> Self {
        Self {
            resources,
            routes,
            redirects,
            metadata,
            site_name,
        }
    }

    /// Empty SiteData constructor.
    pub fn empty() -> Self {
        Self {
            resources: ResourceSet::empty(),
            routes: None,
            redirects: None,
            metadata: None,
            site_name: None,
        }
    }

    // TODO(giac): rename start and reorder the direction of the diff.
    /// Returns the operations to perform to transform the start set into self.
    pub fn diff<'a>(
        &'a self,
        start: &'a SiteData,
        extend_ops: ExtendOps,
    ) -> anyhow::Result<SiteDataDiff<'a>> {
        Ok(SiteDataDiff {
            resource_ops: self.resources.diff(&start.resources),
            route_ops: self.routes_diff(start),
            redirect_ops: self.redirects_diff(start),
            metadata_op: self.metadata_diff(start),
            site_name_op: self.site_name_diff(start),
            extend_ops,
        })
    }

    fn redirects_diff(&self, start: &Self) -> RedirectOps {
        match (&self.redirects, &start.redirects) {
            (Some(r), Some(s)) => r.diff(s),
            (None, Some(_)) => RedirectOps::Replace(Redirects::empty()),
            (Some(s), None) => RedirectOps::Replace(s.clone()),
            _ => RedirectOps::Unchanged,
        }
    }

    fn routes_diff(&self, start: &Self) -> RouteOps {
        match (&self.routes, &start.routes) {
            (Some(r), Some(s)) => r.diff(s),
            (None, Some(_)) => RouteOps::Replace(Routes::empty()),
            (Some(s), None) => RouteOps::Replace(s.clone()),
            _ => RouteOps::Unchanged,
        }
    }

    /// Current logic is to return MetadataOp::Update only when metadata read
    /// from ws-resources is some, and different than the metadata found
    /// on-chain.
    fn metadata_diff(&self, start: &Self) -> MetadataOp {
        if self.metadata != start.metadata && self.metadata.is_some() {
            MetadataOp::Update
        } else {
            MetadataOp::Noop
        }
    }

    fn site_name_diff(&self, start: &Self) -> SiteNameOp {
        if self.site_name.is_some() && self.site_name != start.site_name {
            SiteNameOp::Update
        } else {
            SiteNameOp::Noop
        }
    }

    pub fn resources(&self) -> &ResourceSet {
        &self.resources
    }
}

/// Fetches remote sites.
pub struct RemoteSiteFactory<'a> {
    sui_client: &'a RetriableSuiClient,
    type_origin_map: TypeOriginMap,
}

impl RemoteSiteFactory<'_> {
    /// Creates a new remote site factory.
    pub async fn new(
        sui_client: &RetriableSuiClient,
        package_id: ObjectID,
    ) -> Result<RemoteSiteFactory<'_>> {
        let type_origin_map = sui_client.type_origin_map_for_package(package_id).await?;
        Ok(RemoteSiteFactory {
            sui_client,
            type_origin_map,
        })
    }

    /// Gets the remote site representation stored on chain
    pub async fn get_from_chain(&self, site_id: ObjectID) -> Result<SiteData> {
        let site_fields = self
            .get_site_fields(site_id)
            .await
            .context(format!("Could not fetch fields for site: {site_id}"))?;
        let metadata = Some(site_fields.clone().into());
        let dynamic_fields = self.get_all_dynamic_fields(site_id).await?;
        let resource_path_tag = self.resource_path_tag()?;

        // Chunking ensures that we do not make too many requests at once.
        // TODO(sew-737): multi_get_objects?
        let futures = dynamic_fields.chunks(DF_REQ_BATCH_SIZE).map(|chunk| {
            try_join_all(
                chunk
                    .iter()
                    .filter(|field| field.name.type_ == resource_path_tag)
                    .map(|field| async {
                        self.sui_client
                            .get_sui_object::<ResourceDynamicField>(field.object_id)
                            .await
                            .map(|field| field.value)
                    }),
            )
        });

        let mut resources = ResourceSet::empty();
        let delay = Duration::from_millis(DF_REQ_DELAY_MS);
        let req_s = DF_REQ_BATCH_SIZE as f64 * 1.0 / delay.as_secs_f64();
        tracing::info!(
            batch_size = DF_REQ_BATCH_SIZE,
            ?delay,
            req_s,
            "fetching the resources from the dynamic fields"
        );

        for fut in futures {
            tracing::debug!("fetching a batch of dynamic fields");
            resources.extend(fut.await?);
            tokio::time::sleep(delay).await;
        }

        tracing::debug!("fetching the routes and redirects from the dynamic fields");
        let (routes, redirects) = self.get_routes_and_redirects(site_id).await?;

        Ok(SiteData {
            resources,
            routes,
            redirects,
            metadata,
            site_name: site_fields.name.into(),
        })
    }

    /// Fetches routes and redirects by deriving their DF object IDs from the known keys
    /// and batch-fetching them in a single RPC call.
    async fn get_routes_and_redirects(
        &self,
        site_id: ObjectID,
    ) -> Result<(Option<Routes>, Option<Redirects>)> {
        let vec_u8_tag = TypeTag::Vector(Box::new(TypeTag::U8));

        let routes_df_id = derive_dynamic_field_id(
            site_id,
            &vec_u8_tag,
            &bcs::to_bytes(&ROUTES_DF_KEY.to_vec())?,
        )?;
        let redirects_df_id = derive_dynamic_field_id(
            site_id,
            &vec_u8_tag,
            &bcs::to_bytes(&REDIRECTS_DF_KEY.to_vec())?,
        )?;

        // Batch-fetch both in a single RPC call.
        let responses = self
            .sui_client
            .multi_get_object_with_options(
                &[routes_df_id, redirects_df_id],
                SuiObjectDataOptions::new().with_bcs().with_type(),
            )
            .await?;

        let routes = responses[0]
            .data
            .as_ref()
            .map(|_| {
                contracts::get_sui_object_from_object_response::<SuiDynamicField<Vec<u8>, Routes>>(
                    &responses[0],
                )
            })
            .transpose()?
            .map(|df| df.value);

        let redirects =
            responses[1]
                .data
                .as_ref()
                .map(|_| {
                    contracts::get_sui_object_from_object_response::<
                        SuiDynamicField<Vec<u8>, Redirects>,
                    >(&responses[1])
                })
                .transpose()?
                .map(|df| df.value);

        Ok((routes, redirects))
    }

    /// Gets all the resources and their object ids from chain.
    #[allow(unused)]
    pub async fn get_existing_resources(
        &self,
        object_id: ObjectID,
    ) -> Result<HashMap<String, ObjectID>> {
        let dynamic_fields = self.get_all_dynamic_fields(object_id).await?;
        self.resources_from_dynamic_fields(&dynamic_fields)
    }

    async fn get_all_dynamic_fields(&self, object_id: ObjectID) -> Result<Vec<DynamicFieldInfo>> {
        let iter =
            handle_pagination(|cursor| self.sui_client.get_dynamic_fields(object_id, cursor, None))
                .await?
                .collect();
        Ok(iter)
    }

    async fn get_site_fields(&self, site_id: ObjectID) -> anyhow::Result<SiteFields> {
        Ok(self.sui_client.get_sui_object(site_id).await?)
    }

    /// Filters the dynamic fields to get the resource object IDs.
    #[allow(unused)]
    fn resources_from_dynamic_fields(
        &self,
        dynamic_fields: &[DynamicFieldInfo],
    ) -> Result<HashMap<String, ObjectID>> {
        let type_tag = self.resource_path_tag()?;
        Ok(dynamic_fields
            .iter()
            .filter_map(|field| {
                self.get_path_from_info(field, &type_tag)
                    .map(|path| (path, field.object_id))
            })
            .collect::<HashMap<String, ObjectID>>())
    }

    /// Gets the path of the resource from the dynamic field.
    #[allow(unused)]
    fn get_path_from_info(&self, field: &DynamicFieldInfo, name_tag: &TypeTag) -> Option<String> {
        if field.name.type_ != *name_tag {
            return None;
        }
        field
            .name
            .value
            .as_object()
            .and_then(|obj| obj.get("path"))
            .and_then(|p| p.as_str())
            .map(|s| s.to_owned())
    }

    /// Gets the type tag for the ResourcePath move struct
    fn resource_path_tag(&self) -> Result<TypeTag> {
        contracts::site::ResourcePath
            .to_move_struct_tag_with_type_map(&self.type_origin_map, &[])
            .map(|tag| tag.into())
    }
}

#[cfg(test)]
mod tests {
    use super::SiteData;
    use crate::{
        site::resource::ResourceSet,
        types::{ExtendOps, Metadata, Routes},
    };

    fn routes_from_pair(key: &str, value: &str) -> Option<Routes> {
        Some(Routes(
            vec![(key.to_owned(), value.to_owned())]
                .into_iter()
                .collect(),
        ))
    }

    #[test]
    fn test_routes_diff() {
        let cases = vec![
            (None, None, false),
            (routes_from_pair("a", "b"), None, true),
            (None, routes_from_pair("a", "b"), true),
            (
                routes_from_pair("a", "b"),
                routes_from_pair("a", "b"),
                false,
            ),
            (routes_from_pair("a", "a"), routes_from_pair("a", "b"), true),
        ];

        for (this_routes, other_routes, has_updates) in cases {
            let this = SiteData::new(ResourceSet::empty(), this_routes, None, None, None);
            let other = SiteData::new(ResourceSet::empty(), other_routes, None, None, None);
            let diff = this.diff(&other, ExtendOps::Noop).unwrap();
            assert_eq!(diff.has_updates(), has_updates);
        }
    }

    #[test]
    fn test_metadata_diff() {
        let metadata_empty = Metadata {
            link: None,
            image_url: None,
            description: None,
            project_url: None,
            creator: None,
        };

        let metadata_with_link = |link: &str| -> Metadata {
            Metadata {
                link: Some(link.to_string()),
                image_url: None,
                description: None,
                project_url: None,
                creator: None,
            }
        };
        let cases = vec![
            (Some(Metadata::default()), Some(Metadata::default()), false),
            (Some(Metadata::default()), None, true),
            (None, Some(Metadata::default()), false),
            (Some(metadata_empty), Some(Metadata::default()), true),
            (
                Some(metadata_with_link("https://alink.invalid.org")),
                Some(metadata_with_link("https://blink.invalid.org")),
                true,
            ),
        ];

        for (this_metadata, other_metadata, has_updates) in cases {
            let this = SiteData::new(ResourceSet::empty(), None, None, this_metadata, None);
            let other = SiteData::new(ResourceSet::empty(), None, None, other_metadata, None);
            assert_eq!(
                this.diff(&other, ExtendOps::Noop).unwrap().has_updates(),
                has_updates
            );
        }
    }

    #[test]
    fn test_site_name_diff() {
        let cases = vec![
            (None, None, false),
            (Some("My Walrus Site".to_string()), None, true),
            (None, Some("My Walrus Site".to_string()), false),
            (
                Some("My Walrus Site".to_string()),
                Some("My Walrus Site".to_string()),
                false,
            ),
            (
                Some("My Walrus Site".to_string()),
                Some("My New Walrus Site".to_string()),
                true,
            ),
            (Some("".to_string()), Some("My Site".to_string()), true),
            (Some("My Site".to_string()), Some("".to_string()), true),
            (Some("".to_string()), Some("".to_string()), false),
        ];

        for (this_site_name, other_site_name, has_updates) in cases {
            let this = SiteData::new(ResourceSet::empty(), None, None, None, this_site_name);
            let other = SiteData::new(ResourceSet::empty(), None, None, None, other_site_name);
            assert_eq!(
                this.diff(&other, ExtendOps::Noop).unwrap().has_updates(),
                has_updates
            );
        }
    }
}

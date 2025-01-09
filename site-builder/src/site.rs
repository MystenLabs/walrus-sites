// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

pub mod builder;
pub mod config;
pub mod content;
pub mod contracts;
pub mod manager;
pub mod resource;

use std::{collections::HashMap, time::Duration};

use anyhow::Result;
use contracts::TypeOriginMap;
use futures::future::try_join_all;
use resource::{ResourceOp, ResourceSet};
use sui_types::{base_types::ObjectID, dynamic_field::DynamicFieldInfo, TypeTag};

use crate::{
    publish::WhenWalrusUpload,
    retry_client::RetriableSuiClient,
    summary::SiteDataDiffSummary,
    types::{ResourceDynamicField, RouteOps, Routes, SuiDynamicField},
    util::{handle_pagination, type_origin_map_for_package},
};

pub const SITE_MODULE: &str = "site";

/// The maximum number of dynamic fields to request at once.
const DF_REQ_BATCH_SIZE: usize = 10;
/// The delay between requests for dynamic fields.
const DF_REQ_DELAY_MS: u64 = 100;

/// The diff between two site data.
#[derive(Debug)]
pub struct SiteDataDiff<'a> {
    /// The operations to perform on the resources.
    pub resource_ops: Vec<ResourceOp<'a>>,
    pub route_ops: RouteOps,
}

impl SiteDataDiff<'_> {
    /// Returns `true` if there are updates to be made.
    pub fn has_updates(&self) -> bool {
        self.resource_ops.iter().any(|op| op.is_change()) || !self.route_ops.is_unchanged()
    }

    /// Returns the resources that need to be updated on Walrus.
    pub fn get_walrus_updates(&self, when_upload: &WhenWalrusUpload) -> Vec<&ResourceOp> {
        self.resource_ops
            .iter()
            .filter(|u| u.is_walrus_update(when_upload))
            .collect::<Vec<_>>()
    }

    /// Returns the summary of the operations in the diff.
    pub fn summary(&self, when_upload: &WhenWalrusUpload) -> SiteDataDiffSummary {
        if when_upload.is_always() {
            return SiteDataDiffSummary::from(self);
        }
        SiteDataDiffSummary {
            resource_ops: self
                .resource_ops
                .iter()
                .filter(|op| op.is_change())
                .map(|op| op.into())
                .collect(),
            route_ops: self.route_ops.clone(),
        }
    }
}

/// The site on chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteData {
    resources: ResourceSet,
    routes: Option<Routes>,
}

impl SiteData {
    pub fn new(resources: ResourceSet, routes: Option<Routes>) -> Self {
        Self { resources, routes }
    }

    pub fn empty() -> Self {
        Self {
            resources: ResourceSet::empty(),
            routes: None,
        }
    }

    // TODO(giac): rename start and reorder the direction of the diff.
    /// Returns the operations to perform to transform the start set into self.
    pub fn diff<'a>(&'a self, start: &'a SiteData) -> SiteDataDiff<'a> {
        SiteDataDiff {
            resource_ops: self.resources.diff(&start.resources),
            route_ops: self.routes_diff(start),
        }
    }

    /// Returns the operations to perform to replace all resources in self with the ones in other.
    pub fn replace_all<'a>(&'a self, other: &'a SiteData) -> SiteDataDiff<'a> {
        SiteDataDiff {
            resource_ops: self.resources.replace_all(&other.resources),
            route_ops: self.routes_diff(other),
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
    ) -> Result<RemoteSiteFactory> {
        let type_origin_map = type_origin_map_for_package(sui_client, package_id).await?;
        Ok(RemoteSiteFactory {
            sui_client,
            type_origin_map,
        })
    }

    /// Gets the remote site representation stored on chain
    pub async fn get_from_chain(&self, site_id: ObjectID) -> Result<SiteData> {
        let dynamic_fields = self.get_all_dynamic_fields(site_id).await?;
        let resource_path_tag = self.resource_path_tag()?;

        // Chunking ensures that we do not make too many requests at once.
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

        tracing::debug!("fetching the routes from the dynamic fields");
        let routes = self.get_routes(&dynamic_fields).await?;
        Ok(SiteData { resources, routes })
    }

    async fn get_routes(&self, dynamic_fields: &[DynamicFieldInfo]) -> Result<Option<Routes>> {
        if let Some(routes_field) = dynamic_fields
            .iter()
            .find(|field| field.name.type_ == TypeTag::Vector(Box::new(TypeTag::U8)))
        {
            let routes = self
                .sui_client
                .get_sui_object::<SuiDynamicField<Vec<u8>, Routes>>(routes_field.object_id)
                .await?
                .value;
            Ok(Some(routes))
        } else {
            Ok(None)
        }
    }

    /// Gets all the resources and their object ids from chain.
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

    /// Filters the dynamic fields to get the resource object IDs.
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
    use crate::{site::resource::ResourceSet, types::Routes};

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
            let this = SiteData::new(ResourceSet::empty(), this_routes);
            let other = SiteData::new(ResourceSet::empty(), other_routes);
            let diff = this.diff(&other);
            assert_eq!(diff.has_updates(), has_updates);
        }
    }
}

// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

pub mod builder;
pub mod content;
#[macro_use]
pub mod macros;
pub mod config;
pub mod manager;
pub mod resource;

use std::{collections::HashMap, str::FromStr};

use anyhow::{anyhow, Result};
use resource::{ResourceInfo, ResourceOp, ResourceSet};
use sui_sdk::{
    rpc_types::{SuiMoveValue, SuiObjectDataOptions},
    SuiClient,
};
use sui_types::{base_types::ObjectID, dynamic_field::DynamicFieldInfo, TypeTag};

use crate::util::{get_struct_from_object_response, handle_pagination};

pub const SITE_MODULE: &str = "site";

/// The site on chain.
#[derive(Debug)]
pub struct SiteData {
    resources: ResourceSet,
    // TODO: routes.
}

impl SiteDataDiff<'_> {
    /// Returns `true` if there are updates to be made.
    pub fn has_updates(&self) -> bool {
        !self.resources.is_empty()
    }
}

/// The diff between two site data.
#[derive(Debug)]
pub struct SiteDataDiff<'a> {
    /// The operations to perform on the resources.
    pub resources: Vec<ResourceOp<'a>>,
}

impl SiteData {
    pub fn new(resources: ResourceSet) -> Self {
        Self { resources }
    }

    pub fn empty() -> Self {
        Self {
            resources: ResourceSet::empty(),
        }
    }

    /// Returns the operations to perform to transform the start set into self.
    pub fn diff<'a>(&'a self, start: &'a SiteData) -> SiteDataDiff<'a> {
        SiteDataDiff {
            resources: self.resources.diff(&start.resources),
        }
    }

    /// Returns the operations to perform to replace all resources in self with the ones in other.
    pub fn replace_all<'a>(&'a self, other: &'a SiteData) -> SiteDataDiff<'a> {
        SiteDataDiff {
            resources: self.resources.replace_all(&other.resources),
        }
    }
}

/// Fetches remote sites.
pub struct RemoteSiteFactory<'a> {
    sui_client: &'a SuiClient,
    package_id: ObjectID,
}

impl RemoteSiteFactory<'_> {
    /// Creates a new remote site factory.
    pub fn new(sui_client: &SuiClient, package_id: ObjectID) -> RemoteSiteFactory {
        RemoteSiteFactory {
            sui_client,
            package_id,
        }
    }

    /// Gets the remote site representation stored on chain
    pub async fn get_from_chain(&self, site_id: ObjectID) -> Result<SiteData> {
        let dynamic_fields = self.get_all_dynamic_field_info(site_id).await?;
        let resource_ids = self.resources_from_dynamic_fields(dynamic_fields)?;
        let resources = futures::future::try_join_all(
            resource_ids
                .into_values()
                .map(|id| self.get_remote_resource_info(id)),
        )
        .await?;

        Ok(SiteData {
            resources: ResourceSet::from_iter(resources),
        })
    }

    /// Gets all the resources and their object ids from chain.
    pub async fn get_existing_resources(&self) -> Result<HashMap<String, ObjectID>> {
        let dynamic_fields = self.get_all_dynamic_field_info(self.package_id).await?;
        self.resources_from_dynamic_fields(dynamic_fields)
    }

    async fn get_all_dynamic_field_info(
        &self,
        object_id: ObjectID,
    ) -> Result<Vec<DynamicFieldInfo>> {
        let iter = handle_pagination(|cursor| {
            self.sui_client
                .read_api()
                .get_dynamic_fields(object_id, cursor, None)
        })
        .await?
        .collect();
        Ok(iter)
    }

    /// Get the resource that is hosted on chain at the given object ID.
    async fn get_remote_resource_info(&self, object_id: ObjectID) -> Result<ResourceInfo> {
        let object = get_struct_from_object_response(
            &self
                .sui_client
                .read_api()
                .get_object_with_options(object_id, SuiObjectDataOptions::new().with_content())
                .await?,
        )?;
        get_dynamic_field!(object, "value", SuiMoveValue::Struct)?.try_into()
    }

    /// Filters the dynamic fields to get the resource object IDs.
    fn resources_from_dynamic_fields(
        &self,
        dynamic_fields: Vec<DynamicFieldInfo>,
    ) -> Result<HashMap<String, ObjectID>> {
        let type_tag = self.resource_path_tag();
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
    fn resource_path_tag(&self) -> TypeTag {
        TypeTag::from_str(&format!("{}::{SITE_MODULE}::ResourcePath", self.package_id))
            .expect("this is a valid type tag construction")
    }
}

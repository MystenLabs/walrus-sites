// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Collection of types to mirror the Sui move structs.

use std::collections::BTreeMap;

use move_core_types::u256::U256;
use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize, Serializer};
use sui_types::base_types::ObjectID;

use crate::{
    site::contracts::{self, AssociatedContractStruct, StructTag},
    walrus::types::BlobId,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuiDynamicField<N, V> {
    pub id: ObjectID,
    pub name: N,
    pub value: V,
}

impl<N, V> AssociatedContractStruct for SuiDynamicField<N, V>
where
    N: Serialize + DeserializeOwned,
    V: Serialize + DeserializeOwned,
{
    const CONTRACT_STRUCT: StructTag<'static> = contracts::dynamic_field::Field;
}

pub type ResourceDynamicField = SuiDynamicField<SuiResourcePath, SuiResource>;

/// The name of a path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuiResourcePath(pub String);

impl AssociatedContractStruct for SuiResourcePath {
    const CONTRACT_STRUCT: StructTag<'static> = contracts::site::ResourcePath;
}

/// Information about a resource.
///
/// This struct mirrors the information that is stored on chain.
#[derive(PartialEq, Eq, Debug, Clone, PartialOrd, Ord, Serialize, Deserialize)]
pub(crate) struct SuiResource {
    /// The relative path the resource will have on Sui.
    pub path: String,
    /// Response, Representation and Payload headers.
    #[serde(deserialize_with = "deserialize_http_headers")]
    pub headers: HttpHeaders,
    /// The blob ID of the resource.
    #[serde(serialize_with = "serialize_blob_id")]
    pub blob_id: BlobId,
    /// The hash of the blob contents.
    pub blob_hash: U256,
    /// Byte ranges for the resource.
    pub range: Option<Range>,
}

fn deserialize_http_headers<'de, D>(deserializer: D) -> Result<HttpHeaders, D::Error>
where
    D: Deserializer<'de>,
{
    let headers: Vec<(String, String)> = Deserialize::deserialize(deserializer)?;
    Ok(HttpHeaders(headers.into_iter().collect()))
}

/// Serialize as string to make sure that the json output uses the base64 encoding.
fn serialize_blob_id<S>(blob_id: &BlobId, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.collect_str(blob_id)
}

impl AssociatedContractStruct for SuiResource {
    const CONTRACT_STRUCT: StructTag<'static> = contracts::site::Resource;
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct HttpHeaders(pub BTreeMap<String, String>);

/// The routes of a site.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Routes(pub BTreeMap<String, String>);

impl Routes {
    pub fn empty() -> Self {
        Routes(BTreeMap::new())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Checks if the routes are different.
    pub fn diff(&self, start: &Self) -> RouteOps {
        if self.0 == start.0 {
            RouteOps::Unchanged
        } else {
            RouteOps::Replace(self.clone())
        }
    }
}

impl AssociatedContractStruct for Routes {
    const CONTRACT_STRUCT: StructTag<'static> = contracts::site::Routes;
}

#[derive(Debug, Clone)]
pub enum RouteOps {
    Unchanged,
    Replace(Routes),
}

impl RouteOps {
    pub fn is_unchanged(&self) -> bool {
        matches!(self, RouteOps::Unchanged)
    }
}

/// Range of bytes for a resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub struct Range {
    pub start: Option<u64>,
    pub end: Option<u64>,
}

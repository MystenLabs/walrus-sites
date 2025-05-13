// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Collection of types to mirror the Sui move structs.

use std::{
    borrow::Borrow,
    collections::{btree_map, BTreeMap},
    str::FromStr,
};

use move_core_types::u256::U256;
use serde::{de::DeserializeOwned, Deserialize, Serialize, Serializer};
use sui_types::{
    base_types::{ObjectID, SuiAddress},
    id::UID,
};

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
    pub headers: HttpHeaders,
    /// The blob ID of the resource.
    #[serde(serialize_with = "serialize_blob_id")]
    pub blob_id: BlobId,
    /// The hash of the blob contents.
    pub blob_hash: U256,
    /// Byte ranges for the resource.
    pub range: Option<Range>,
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

/// The representation of a move VecMap.
#[derive(Serialize, Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Default)]
pub struct VecMap<K, V>(pub BTreeMap<K, V>);

impl<K, V> VecMap<K, V>
where
    K: Ord,
{
    pub fn new() -> Self {
        VecMap(BTreeMap::new())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn entry(&mut self, key: K) -> btree_map::Entry<K, V> {
        self.0.entry(key)
    }

    #[allow(unused)]
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.0.insert(key, value)
    }

    #[allow(unused)]
    pub fn or_insert(&mut self, key: K, value: V) -> &mut V {
        self.0.entry(key).or_insert(value)
    }

    #[allow(unused)]
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q> + Ord,
        Q: ?Sized + Ord,
    {
        self.0.contains_key(key)
    }

    #[allow(unused)]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn iter(&self) -> btree_map::Iter<K, V> {
        self.0.iter()
    }

    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q> + Ord,
        Q: ?Sized + Ord,
    {
        self.0.get(key)
    }
}

impl<K, V> IntoIterator for VecMap<K, V> {
    type Item = (K, V);
    type IntoIter = <BTreeMap<K, V> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'de, K, V> Deserialize<'de> for VecMap<K, V>
where
    K: Deserialize<'de> + Ord,
    V: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let routes: BTreeMap<K, V> = Deserialize::deserialize(deserializer)?;
            Ok(Self(routes))
        } else {
            let routes: Vec<(K, V)> = Deserialize::deserialize(deserializer)?;
            Ok(Self(routes.into_iter().collect()))
        }
    }
}

impl<K, V> FromIterator<(K, V)> for VecMap<K, V>
where
    K: Ord,
{
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        VecMap(iter.into_iter().collect())
    }
}

/// The representation of the HTTP headers.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct HttpHeaders(pub VecMap<String, String>);

/// The routes of a site.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Routes(pub VecMap<String, String>);

impl Routes {
    pub fn empty() -> Self {
        Routes(VecMap::new())
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Metadata {
    pub link: Option<String>,
    pub image_url: Option<String>,
    pub description: Option<String>,
    pub project_url: Option<String>,
    pub creator: Option<String>,
}

impl Default for Metadata {
    fn default() -> Self {
        Self {
            link: None,
            image_url: Some("https://www.walrus.xyz/walrus-site".to_string()),
            description: Some("A walrus site created using Walrus and Sui!".to_string()),
            project_url: None,
            creator: None,
        }
    }
}

impl From<SiteFields> for Metadata {
    fn from(value: SiteFields) -> Self {
        let SiteFields {
            link,
            image_url,
            description,
            project_url,
            creator,
            ..
        } = value;
        Metadata {
            link,
            image_url,
            description,
            project_url,
            creator,
        }
    }
}

#[derive(Debug, Clone)]
pub enum MetadataOp {
    Update,
    Noop,
}

impl MetadataOp {
    pub fn is_noop(&self) -> bool {
        matches!(self, Self::Noop)
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct SiteFields {
    #[allow(dead_code)]
    pub id: UID,
    #[allow(dead_code)]
    pub name: String,
    pub link: Option<String>,
    pub image_url: Option<String>,
    pub description: Option<String>,
    pub project_url: Option<String>,
    pub creator: Option<String>,
}

impl AssociatedContractStruct for SiteFields {
    const CONTRACT_STRUCT: StructTag<'static> = contracts::site::Site;
}

// SuiNS definitions

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NameRecord {
    pub nft_id: ObjectID,
    /// Timestamp in milliseconds when the record expires.
    pub expiration_timestamp_ms: u64,
    /// The target address that this domain points to
    pub target_address: Option<SuiAddress>,
    /// Additional data which may be stored in a record
    pub data: VecMap<String, String>,
}

impl NameRecord {
    /// Returns the `walrus_site_id` for the record, if it exists.
    ///
    /// If it does not exist, it returns the ID of the wallet pointed to the by the record.
    pub(crate) fn walrus_site_id(&self) -> Option<ObjectID> {
        self.data
            .get("walrus_site_id")
            .and_then(|s| ObjectID::from_str(s).ok())
            .or_else(|| {
                tracing::info!("no walrus_site_id found in the record");
                self.target_address.map(|address| address.into())
            })
    }
}

/// The `Domain` type for keying the record table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Domain {
    pub labels: Vec<String>,
}

impl Domain {
    /// Returns the normalized SuiNS name, if valid.
    pub(crate) fn from_name(name: &str) -> Option<Self> {
        let name = name.trim();
        if name.is_empty() {
            return None;
        }

        // TODO: Check if the name is specified as `@<name>`
        if !name.ends_with(".sui") {
            return None;
        }

        let labels = name
            .split('.')
            .rev()
            .map(|s| s.to_owned())
            .collect::<Vec<_>>();
        Some(Domain { labels })
    }
}

impl AssociatedContractStruct for NameRecord {
    const CONTRACT_STRUCT: StructTag<'static> = contracts::suins::NameRecord;
}

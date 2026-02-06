// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Collection of types to mirror the Sui move structs.
use std::{
    borrow::Borrow,
    collections::{btree_map, BTreeMap, HashMap},
    num::NonZeroU16,
    ops::Deref,
    str::FromStr,
};

use move_core_types::u256::U256;
use serde::{de::DeserializeOwned, Deserialize, Serialize, Serializer};
use sui_types::{
    base_types::{ObjectID, ObjectRef, SuiAddress},
    id::UID,
};
use walrus_sdk::sui::contracts::StructTag;

use crate::{
    site::contracts::{self, AssociatedContractStruct},
    util::deserialize_bag_or_table,
    walrus::types::BlobId,
};

pub type ObjectCache = HashMap<ObjectID, ObjectRef>;

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
pub struct SuiResource {
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

    pub fn entry(&mut self, key: K) -> btree_map::Entry<'_, K, V> {
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

    pub fn iter(&self) -> btree_map::Iter<'_, K, V> {
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
impl Deref for HttpHeaders {
    type Target = VecMap<String, String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

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

#[derive(Debug, Clone, Copy)]
pub enum SiteNameOp {
    Update,
    Noop,
}

impl SiteNameOp {
    pub fn is_noop(&self) -> bool {
        matches!(self, Self::Noop)
    }
}

#[derive(Debug, Clone)]
pub enum ExtendOps {
    Extend {
        total_wal_cost: u64,
        blobs_epochs: Vec<(ObjectRef, u32)>,
    },
    Noop,
}
impl ExtendOps {
    pub(crate) fn is_noop(&self) -> bool {
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
/// Sui type for staking object
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Staking {
    /// Object id of the Sui object.
    pub id: ObjectID,
    /// The version of the staking object.
    pub version: u64,
    /// The package ID of the staking object.
    pub package_id: ObjectID,
    /// The new package ID of the staking object.
    pub(crate) new_package_id: Option<ObjectID>,
    /// The inner staking state.
    pub(crate) inner: StakingInnerV1,
}

impl AssociatedContractStruct for StakingObjectForDeserialization {
    const CONTRACT_STRUCT: StructTag<'static> = contracts::staking::Staking;
}

type CommitteeShardAssignment = Vec<(ObjectID, Vec<u16>)>;

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub(crate) struct EpochParams {
    /// The storage capacity of the system.
    total_capacity_size: u64,
    /// The price per unit size of storage.
    storage_price_per_unit_size: u64,
    /// The write price per unit size.
    write_price_per_unit_size: u64,
}

/// The epoch state.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum EpochState {
    /// The epoch change is currently in progress.
    ///
    /// Contains the weight of the nodes that have already attested that they finished the sync.
    EpochChangeSync(u16),
    /// The epoch change has been completed at the contained timestamp.
    #[serde(deserialize_with = "chrono::serde::ts_milliseconds::deserialize")]
    EpochChangeDone(chrono::DateTime<chrono::Utc>),
    /// The parameters for the next epoch have been selected.
    ///
    /// The contained timestamp is the start of the current epoch.
    #[serde(deserialize_with = "chrono::serde::ts_milliseconds::deserialize")]
    NextParamsSelected(chrono::DateTime<chrono::Utc>),
}

/// Sui type for inner staking object
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub(crate) struct StakingInnerV1 {
    /// The number of shards in the system.
    pub(crate) n_shards: NonZeroU16,
    /// The duration of an epoch in ms. Does not affect the first (zero) epoch.
    pub(crate) epoch_duration: u64,
    /// Special parameter, used only for the first epoch. The timestamp when the
    /// first epoch can be started.
    pub(crate) first_epoch_start: u64,
    /// Object ID of the object table storing the staking pools.
    #[serde(deserialize_with = "deserialize_bag_or_table")]
    pub(crate) pools: ObjectID,
    /// The current epoch of the Walrus system.
    pub(crate) epoch: u32,
    /// Stores the active set of storage nodes. Provides automatic sorting and
    /// tracks the total amount of staked WAL.
    pub(crate) active_set: ObjectID,
    /// The next committee in the system.
    pub(crate) next_committee: Option<CommitteeShardAssignment>,
    /// The current committee in the system.
    pub(crate) committee: CommitteeShardAssignment,
    /// The previous committee in the system.
    pub(crate) previous_committee: CommitteeShardAssignment,
    /// The next epoch parameters.
    pub(crate) next_epoch_params: Option<EpochParams>,
    /// The state of the current epoch.
    pub(crate) epoch_state: EpochState,
    /// Extended field holding public keys for the next epoch.
    pub(crate) next_epoch_public_keys: ObjectID,
}

impl AssociatedContractStruct for StakingInnerV1 {
    const CONTRACT_STRUCT: StructTag<'static> = contracts::staking_inner::StakingInnerV1;
}

/// Sui type for outer staking object. Used for deserialization.
#[derive(Debug, Clone, Deserialize)]
pub struct StakingObjectForDeserialization {
    pub id: ObjectID,
    pub version: u64,
    pub package_id: ObjectID,
    pub new_package_id: Option<ObjectID>,
}

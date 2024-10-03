// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Functionality to read and check the files in of a website.

use std::{
    cmp::Ordering,
    collections::{BTreeSet, HashMap},
    fmt::{self, Display},
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};
use fastcrypto::hash::{HashFunction, Sha256};
use flate2::{write::GzEncoder, Compression};
use move_core_types::u256::U256;
use sui_sdk::rpc_types::{SuiMoveStruct, SuiMoveValue};

use crate::{
    site::{config::WSResources, content::ContentType},
    walrus::{types::BlobId, Walrus},
};

/// Information about a resource.
///
/// This struct mirrors the information that is stored on chain.
#[derive(PartialEq, Eq, Debug, Clone, PartialOrd, Ord)]
pub(crate) struct ResourceInfo {
    /// The relative path the resource will have on Sui.
    pub path: String,
    /// Response, Representation and Payload headers.
    pub headers: HttpHeaders,
    /// The blob ID of the resource.
    pub blob_id: BlobId,
    /// The hash of the blob contents.
    pub blob_hash: U256,
}

impl TryFrom<&SuiMoveStruct> for ResourceInfo {
    type Error = anyhow::Error;

    fn try_from(source: &SuiMoveStruct) -> Result<Self, Self::Error> {
        let path = get_dynamic_field!(source, "path", SuiMoveValue::String)?;
        let headers: HttpHeaders =
            get_dynamic_field!(source, "headers", SuiMoveValue::Struct)?.try_into()?;
        let blob_id = blob_id_from_u256(
            get_dynamic_field!(source, "blob_id", SuiMoveValue::String)?.parse::<U256>()?,
        );
        let blob_hash =
            get_dynamic_field!(source, "blob_hash", SuiMoveValue::String)?.parse::<U256>()?;

        Ok(Self {
            path,
            headers,
            blob_id,
            blob_hash,
        })
    }
}

#[derive(serde::Serialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct HttpHeader {
    pub name: String,
    pub value: String,
}

#[derive(serde::Serialize, Clone, Debug, PartialEq, Eq)]
pub struct HttpHeaders(pub HashMap<String, String>);

impl PartialOrd for HttpHeaders {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HttpHeaders {
    fn cmp(&self, other: &Self) -> Ordering {
        let self_sorted: Vec<_> = self.0.iter().collect();
        let other_sorted: Vec<_> = other.0.iter().collect();
        self_sorted.cmp(&other_sorted)
    }
}

impl From<HashMap<String, String>> for HttpHeaders {
    fn from(values: HashMap<String, String>) -> Self {
        Self(values)
    }
}

#[derive(Debug)]
struct VecMapContents {
    pub entries: Vec<VecMapEntry>,
}

impl TryFrom<Vec<SuiMoveValue>> for VecMapContents {
    type Error = anyhow::Error;

    fn try_from(entries: Vec<SuiMoveValue>) -> std::result::Result<Self, Self::Error> {
        let mut contents = Vec::new();
        for entry in entries {
            if let SuiMoveValue::Struct(s) = entry {
                let key = get_dynamic_field!(s, "key", SuiMoveValue::String)?;
                let value = get_dynamic_field!(s, "value", SuiMoveValue::String)?;
                contents.push(VecMapEntry { key, value });
            } else {
                return Err(anyhow!("Expected SuiMoveValue::Struct"));
            }
        }
        Ok(VecMapContents { entries: contents })
    }
}

#[derive(Debug)]
struct VecMapEntry {
    pub key: String,
    pub value: String,
}

impl TryFrom<SuiMoveStruct> for HttpHeaders {
    type Error = anyhow::Error;

    fn try_from(source: SuiMoveStruct) -> Result<Self, Self::Error> {
        let contents: VecMapContents =
            get_dynamic_field!(source, "contents", SuiMoveValue::Vector)?.try_into()?;
        let mut headers = HashMap::new();
        for entry in contents.entries {
            headers.insert(entry.key, entry.value);
        }
        Ok(Self(headers))
    }
}

impl TryFrom<SuiMoveStruct> for ResourceInfo {
    type Error = anyhow::Error;

    fn try_from(value: SuiMoveStruct) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

pub(crate) fn blob_id_from_u256(input: U256) -> BlobId {
    BlobId(input.to_le_bytes())
}

/// The resource that is to be created or updated on Sui.
///
/// This struct contains additional information that is not stored on chain, compared to
/// [`ResourceInfo`] (`unencoded_size`, `full_path`).
///
/// [`Resource`] objects are always compared on their `info` field
/// ([`ResourceInfo`]), and never on their `unencoded_size` or `full_path`.
#[derive(PartialEq, Eq, Debug, Clone)]
pub(crate) struct Resource {
    pub info: ResourceInfo,
    /// The unencoded length of the resource.
    pub unencoded_size: usize,
    /// The full path of the resource on disk.
    pub full_path: PathBuf,
}

impl PartialOrd for Resource {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Resource {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.info.cmp(&other.info)
    }
}

impl From<ResourceInfo> for Resource {
    fn from(source: ResourceInfo) -> Self {
        Self {
            info: source,
            unencoded_size: 0,
            full_path: PathBuf::default(),
        }
    }
}

impl Resource {
    pub fn new(
        resource_path: String,
        full_path: PathBuf,
        headers: HttpHeaders,
        blob_id: BlobId,
        blob_hash: U256,
        unencoded_size: usize,
    ) -> Self {
        Resource {
            info: ResourceInfo {
                path: resource_path,
                headers,
                blob_id,
                blob_hash,
            },
            unencoded_size,
            full_path,
        }
    }
}

impl Display for Resource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Resource: {:?}, blob id: {})",
            self.info.path, self.info.blob_id
        )
    }
}

/// The operations on resources that are necessary to update a site.
///
/// Updates to resources are implemented as deleting the outdated
/// resource and adding a new one. Two [`Resources`][Resource] are
/// different if their respective [`ResourceInfo`] differ.
pub enum ResourceOp<'a> {
    Deleted(&'a Resource),
    Created(&'a Resource),
}

impl<'a> fmt::Debug for ResourceOp<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (op, path) = match self {
            ResourceOp::Deleted(resource) => ("delete", &resource.info.path),
            ResourceOp::Created(resource) => ("create", &resource.info.path),
        };
        f.debug_struct("ResourceOp")
            .field("operation", &op)
            .field("path", path)
            .finish()
    }
}

impl<'a> ResourceOp<'a> {
    /// Returns the resource for which this operation is defined.
    pub fn inner(&self) -> &'a Resource {
        match self {
            ResourceOp::Deleted(resource) => resource,
            ResourceOp::Created(resource) => resource,
        }
    }
}

/// A summary of the operations performed by the site builder.
#[derive(Debug, Clone)]
pub(crate) struct OperationSummary {
    operation: String,
    path: String,
    blob_id: BlobId,
}

impl<'a> From<&ResourceOp<'a>> for OperationSummary {
    fn from(source: &ResourceOp<'a>) -> Self {
        let (op, info) = match source {
            ResourceOp::Deleted(resource) => ("deleted".to_owned(), &resource.info),
            ResourceOp::Created(resource) => ("created".to_owned(), &resource.info),
        };
        OperationSummary {
            operation: op,
            path: info.path.clone(),
            blob_id: info.blob_id,
        }
    }
}

impl Display for OperationSummary {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} resource {} with blob ID {}",
            self.operation, self.path, self.blob_id
        )
    }
}

pub(crate) struct OperationsSummary(pub Vec<OperationSummary>);

impl<'a> From<&Vec<ResourceOp<'a>>> for OperationsSummary {
    fn from(source: &Vec<ResourceOp<'a>>) -> Self {
        Self(source.iter().map(OperationSummary::from).collect())
    }
}

impl<'a> From<Vec<ResourceOp<'a>>> for OperationsSummary {
    fn from(source: Vec<ResourceOp<'a>>) -> Self {
        Self::from(&source)
    }
}

impl Display for OperationsSummary {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.0.is_empty() {
            return write!(f, "No operations need to be performed.");
        }
        let ops = self
            .0
            .iter()
            .map(|s| format!("  - {}", s))
            .collect::<Vec<_>>()
            .join("\n");
        write!(f, "Operations performed:\n{}", ops)
    }
}

/// A set of resources composing a site.
#[derive(Default, Debug, Clone)]
pub(crate) struct ResourceSet {
    pub inner: BTreeSet<Resource>,
}

impl ResourceSet {
    /// Returns a vector of deletion and creation operations to move
    /// from the current set to the target set.
    ///
    /// The deletions are always before the creation operations, such
    /// that if two resources have the same path but different
    /// contents they are first deleted and then created anew.
    pub fn diff<'a>(&'a self, target: &'a ResourceSet) -> Vec<ResourceOp<'a>> {
        let create = self
            .inner
            .difference(&target.inner)
            .map(ResourceOp::Created);
        let delete = target
            .inner
            .difference(&self.inner)
            .map(ResourceOp::Deleted);
        delete.chain(create).collect()
    }

    /// Returns a vector of operations to delete all resources in the set.
    pub fn delete_all(&self) -> Vec<ResourceOp> {
        self.inner.iter().map(ResourceOp::Deleted).collect()
    }

    /// Returns a vector of operations to create all resources in the set.
    pub fn create_all(&self) -> Vec<ResourceOp> {
        self.inner.iter().map(ResourceOp::Created).collect()
    }

    /// Returns a vector of operations to replace the resources in `self` with the ones in `other`.
    pub fn replace_all<'a>(&'a self, other: &'a ResourceSet) -> Vec<ResourceOp<'a>> {
        // Delete all the resources already on chain.
        let mut delete_operations = self.delete_all();
        // Create all the resources on disk.
        let create_operations = other.create_all();
        delete_operations.extend(create_operations);
        delete_operations
    }
}

impl FromIterator<Resource> for ResourceSet {
    fn from_iter<I: IntoIterator<Item = Resource>>(source: I) -> Self {
        Self {
            inner: BTreeSet::from_iter(source),
        }
    }
}

impl FromIterator<ResourceInfo> for ResourceSet {
    fn from_iter<I: IntoIterator<Item = ResourceInfo>>(source: I) -> Self {
        Self::from_iter(source.into_iter().map(Resource::from))
    }
}

impl Display for ResourceSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ResourceSet({})",
            self.inner
                .iter()
                .map(|r| r.info.path.to_owned())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

/// Loads and manages the set of resources composing the site.
#[derive(Debug)]
pub(crate) struct ResourceManager {
    /// The controller for the Walrus CLI.
    pub walrus: Walrus,
    /// The resources in the site.
    pub resources: ResourceSet,
    /// The ws-resources.json contents.
    pub ws_resources: Option<WSResources>,
    /// The location of the ws-resources.json.
    pub ws_resources_path: PathBuf,
}

impl ResourceManager {
    pub fn new(walrus: Walrus, ws_resources_path: PathBuf) -> Result<Self> {
        Ok(ResourceManager {
            walrus,
            resources: ResourceSet::default(),
            ws_resources: None,
            ws_resources_path,
        })
    }

    /// Read a resource at a path.
    ///
    /// Ignores empty files.
    pub fn read_resource(&self, full_path: &Path, root: &Path) -> Result<Option<Resource>> {
        let resource_path = full_path_to_resource_path(full_path, root)?;
        let mut http_headers: HashMap<String, String> = self
            .ws_resources
            .as_ref()
            .and_then(|config| config.headers.as_ref())
            .and_then(|headers| headers.get(&resource_path))
            .cloned()
            // Cast the keys to lowercase because http headers
            //  are case-insensitive: RFC7230 sec. 2.7.3
            .map(|headers| {
                headers
                    .into_iter()
                    .map(|(k, v)| (k.to_lowercase(), v))
                    .collect()
            })
            .unwrap_or_default();

        let extension = full_path
            .extension()
            .unwrap_or(
                full_path
                    .file_name()
                    .expect("the path should not terminate in `..`"),
            )
            .to_str();

        // Is Content-Encoding specified? Else, add default to headers.
        http_headers
            .entry("content-encoding".to_string())
            .or_insert_with(||
                // Currently we only support this (plaintext) content encoding
                // so no need to parse it as we do with content-type.
                "identity".to_string());

        // Read the content type.
        let content_type =
            ContentType::try_from_extension(extension.ok_or_else(|| {
                anyhow!("Could not read file extension for {}", full_path.display())
            })?)
            .unwrap_or(ContentType::TextHtml); // Default ContentType.

        // If content-type not specified in ws-resources.yaml, parse it from the extension.
        http_headers
            .entry("content-type".to_string())
            .or_insert_with(|| content_type.to_string());

        let plain_content: Vec<u8> = std::fs::read(full_path)?;
        // TODO(giac): this could be (i) async; (ii) pre configured with the number of shards to
        //     avoid chain interaction (maybe after adding `info` to the JSON commands).
        let output = self.walrus.blob_id(full_path.to_owned(), None)?;

        // Hash the contents of the file - this will be contained in the site::Resource
        // to verify the integrity of the blob when fetched from an aggregator.
        let mut hash_function = Sha256::default();
        hash_function.update(&plain_content);
        let blob_hash: [u8; 32] = hash_function.finalize().digest;

        Ok(Some(Resource::new(
            resource_path,
            full_path.to_owned(),
            HttpHeaders::from(http_headers),
            output.blob_id,
            U256::from_le_bytes(&blob_hash),
            // TODO(giac): Change to `content.len()` when the problem with content encoding is
            // fixed.
            plain_content.len(),
        )))
    }

    /// Recursively iterate a directory and load all [`Resources`][Resource] within.
    pub fn read_dir(&mut self, root: &Path) -> Result<()> {
        let cli_path = self.ws_resources_path.as_path();
        if !cli_path.exists() {
            eprintln!(
                "Warning: The specified ws-resources.json path does not exist: {}.
                Continuing without it...",
                cli_path.display()
            );
            self.ws_resources = None;
        }
        self.ws_resources = WSResources::read(cli_path).ok();
        self.resources = ResourceSet::from_iter(self.iter_dir(root, root)?);
        Ok(())
    }

    fn iter_dir(&self, start: &Path, root: &Path) -> Result<Vec<Resource>> {
        let mut resources: Vec<Resource> = vec![];
        let entries = fs::read_dir(start)?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                resources.extend(self.iter_dir(&path, root)?);
            } else if let Some(res) = self.read_resource(&path, root).context(format!(
                "error while reading resource `{}`",
                path.to_string_lossy()
            ))? {
                resources.push(res);
            }
        }
        Ok(resources)
    }
}

// TODO(giac): remove allow after getting compression back.
#[allow(dead_code)]
fn compress(content: &[u8]) -> Result<Vec<u8>> {
    if content.is_empty() {
        // Compression of an empty vector may result in compression headers
        return Ok(vec![]);
    }
    let mut encoder = GzEncoder::new(vec![], Compression::default());
    encoder.write_all(content)?;
    Ok(encoder.finish()?)
}

/// Converts the full path of the resource to the on-chain resource path.
pub(crate) fn full_path_to_resource_path(full_path: &Path, root: &Path) -> Result<String> {
    let rel_path = full_path.strip_prefix(root)?;
    Ok(format!(
        "/{}",
        rel_path
            .to_str()
            .ok_or(anyhow!("could not process the path string: {:?}", rel_path))?
    ))
}

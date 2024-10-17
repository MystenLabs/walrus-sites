// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Functionality to read and check the files in of a website.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::{self, Display},
    fs,
    io::Write,
    num::NonZeroU16,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};
use fastcrypto::hash::{HashFunction, Sha256};
use flate2::{write::GzEncoder, Compression};
use futures::future::try_join_all;
use move_core_types::u256::U256;

use super::SiteData;
use crate::{
    publish::WhenWalrusUpload,
    site::{config::WSResources, content::ContentType},
    types::{HttpHeaders, SuiResource},
    walrus::{types::BlobId, Walrus},
};

/// The resource that is to be created or updated on Sui.
///
/// This struct contains additional information that is not stored on chain, compared to
/// [`SuiResource`] (`unencoded_size`, `full_path`).
///
/// [`Resource`] objects are always compared on their `info` field
/// ([`SuiResource`]), and never on their `unencoded_size` or `full_path`.
#[derive(PartialEq, Eq, Debug, Clone)]
pub(crate) struct Resource {
    pub info: SuiResource,
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

impl From<SuiResource> for Resource {
    fn from(source: SuiResource) -> Self {
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
            info: SuiResource {
                path: resource_path,
                headers,
                blob_id,
                blob_hash,
                // TODO(giac): eventually implement resource bundling.
                range: None,
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
/// different if their respective [`SuiResource`] differ.
pub enum ResourceOp<'a> {
    Deleted(&'a Resource),
    Created(&'a Resource),
    Unchanged(&'a Resource),
}

impl<'a> fmt::Debug for ResourceOp<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (op, path) = match self {
            ResourceOp::Deleted(resource) => ("delete", &resource.info.path),
            ResourceOp::Created(resource) => ("create", &resource.info.path),
            ResourceOp::Unchanged(resource) => ("unchanged", &resource.info.path),
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
            ResourceOp::Unchanged(resource) => resource,
        }
    }

    /// Returns if the operation needs to be uploaded to Walrus.
    pub fn is_walrus_update(&self, when_upload: &WhenWalrusUpload) -> bool {
        matches!(self, ResourceOp::Created(_))
            || (when_upload.is_always() && matches!(self, ResourceOp::Unchanged(_)))
    }

    /// Returns true if the operation modifies a resource.
    pub fn is_change(&self) -> bool {
        matches!(self, ResourceOp::Created(_) | ResourceOp::Deleted(_))
    }
}

/// A summary of the operations performed by the site builder.
#[derive(Debug, Clone)]
pub(crate) struct ResourceOpSummary {
    operation: String,
    path: String,
    blob_id: BlobId,
}

impl<'a> From<&ResourceOp<'a>> for ResourceOpSummary {
    fn from(source: &ResourceOp<'a>) -> Self {
        let (op, info) = match source {
            ResourceOp::Deleted(resource) => ("deleted".to_owned(), &resource.info),
            ResourceOp::Created(resource) => ("created".to_owned(), &resource.info),
            ResourceOp::Unchanged(resource) => ("unchanged".to_owned(), &resource.info),
        };
        ResourceOpSummary {
            operation: op,
            path: info.path.clone(),
            blob_id: info.blob_id,
        }
    }
}

impl Display for ResourceOpSummary {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} resource {} with blob ID {}",
            self.operation, self.path, self.blob_id
        )
    }
}

pub(crate) struct OperationsSummary(pub Vec<ResourceOpSummary>);

impl<'a> From<&Vec<ResourceOp<'a>>> for OperationsSummary {
    fn from(source: &Vec<ResourceOp<'a>>) -> Self {
        Self(source.iter().map(ResourceOpSummary::from).collect())
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
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResourceSet {
    pub inner: BTreeSet<Resource>,
}

impl ResourceSet {
    /// Creates an empty resource set.
    pub fn empty() -> Self {
        Self {
            inner: BTreeSet::new(),
        }
    }

    /// Returns a vector of deletion and creation operations to move
    /// from the start set to the current set.
    ///
    /// The deletions are always before the creation operations, such
    /// that if two resources have the same path but different
    /// contents they are first deleted and then created anew.
    pub fn diff<'a>(&'a self, start: &'a ResourceSet) -> Vec<ResourceOp<'a>> {
        let create = self.inner.difference(&start.inner).map(ResourceOp::Created);
        let delete = start.inner.difference(&self.inner).map(ResourceOp::Deleted);
        let unchanged = self
            .inner
            .intersection(&start.inner)
            .map(ResourceOp::Unchanged);
        delete.chain(create).chain(unchanged).collect()
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

impl FromIterator<SuiResource> for ResourceSet {
    fn from_iter<I: IntoIterator<Item = SuiResource>>(source: I) -> Self {
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
    /// The ws-resources.json contents.
    pub ws_resources: Option<WSResources>,
    /// The ws-resource file path.
    pub ws_resources_path: Option<PathBuf>,
    /// The number of shards of the Walrus system.
    pub n_shards: NonZeroU16,
}

impl ResourceManager {
    pub async fn new(
        walrus: Walrus,
        ws_resources: Option<WSResources>,
        ws_resources_path: Option<PathBuf>,
    ) -> Result<Self> {
        let n_shards = walrus.info(false).await?.n_shards;
        Ok(ResourceManager {
            walrus,
            ws_resources,
            ws_resources_path,
            n_shards,
        })
    }

    /// Read a resource at a path.
    ///
    /// Ignores empty files.
    pub async fn read_resource(&self, full_path: &Path, root: &Path) -> Result<Option<Resource>> {
        if let Some(ws_path) = &self.ws_resources_path {
            if full_path == ws_path {
                tracing::debug!(?full_path, "ignoring the ws-resources config file");
                return Ok(None);
            }
        }

        let resource_path = full_path_to_resource_path(full_path, root)?;
        let mut http_headers: BTreeMap<String, String> = self
            .ws_resources
            .as_ref()
            .and_then(|config| config.headers.as_ref())
            .and_then(|headers| headers.get(&resource_path))
            .cloned()
            // Cast the keys to lowercase because http headers
            //  are case-insensitive: RFC7230 sec. 2.7.3
            .map(|headers| {
                headers
                    .0
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
            .or_insert(
                // Currently we only support this (plaintext) content encoding
                // so no need to parse it as we do with content-type.
                "identity".to_string(),
            );

        // Read the content type.
        let content_type =
            ContentType::try_from_extension(extension.ok_or_else(|| {
                anyhow!("Could not read file extension for {}", full_path.display())
            })?)
            .unwrap_or(ContentType::ApplicationOctetstream); // Default ContentType.

        // If content-type not specified in ws-resources.yaml, parse it from the extension.
        http_headers
            .entry("content-type".to_string())
            .or_insert(content_type.to_string());

        let plain_content: Vec<u8> = std::fs::read(full_path)?;
        let output = self
            .walrus
            .blob_id(full_path.to_owned(), Some(self.n_shards))
            .await
            .context(format!(
                "error while computing the blob id for path: {}",
                full_path.to_string_lossy()
            ))?;

        // Hash the contents of the file - this will be contained in the site::Resource
        // to verify the integrity of the blob when fetched from an aggregator.
        let mut hash_function = Sha256::default();
        hash_function.update(&plain_content);
        let blob_hash: [u8; 32] = hash_function.finalize().digest;

        Ok(Some(Resource::new(
            resource_path,
            full_path.to_owned(),
            HttpHeaders(http_headers),
            output.blob_id,
            U256::from_le_bytes(&blob_hash),
            plain_content.len(),
        )))
    }

    /// Recursively iterate a directory and load all [`Resources`][Resource] within.
    pub async fn read_dir(&mut self, root: &Path) -> Result<SiteData> {
        let resource_paths = Self::iter_dir(root, root)?;
        let resources = ResourceSet::from_iter(
            try_join_all(
                resource_paths
                    .iter()
                    .map(|(full_path, root)| self.read_resource(full_path, root)),
            )
            .await
            .context("error in loading one of the resources")?
            .into_iter()
            .flatten(),
        );

        Ok(SiteData::new(
            resources,
            self.ws_resources
                .as_ref()
                .and_then(|config| config.routes.clone()),
        ))
    }

    fn iter_dir(start: &Path, root: &Path) -> Result<Vec<(PathBuf, PathBuf)>> {
        let mut resources = vec![];
        let entries = fs::read_dir(start)?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                resources.extend(Self::iter_dir(&path, root)?);
            } else {
                resources.push((path.to_owned(), root.to_owned()));
            }
        }
        Ok(resources)
    }
}

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

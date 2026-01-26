// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Functionality to read and check the files in of a website.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::{self, Debug, Display},
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Context, Result};
use fastcrypto::hash::{HashFunction, Sha256};
use flate2::{write::GzEncoder, Compression};
use move_core_types::u256::U256;

use super::SiteData;
use crate::{
    args::ResourcePaths,
    display,
    site::{config::WSResources, content::ContentType},
    types::{HttpHeaders, SuiResource, VecMap},
    util::{is_ignored, is_pattern_match},
    walrus::{command::QuiltBlobInput, types::BlobId},
};

#[path = "../unit_tests/site.resource.tests.rs"]
#[cfg(test)]
mod resource_tests;

/// Maximum size (in bytes) for a BCS-serialized identifier in a quilt.
pub const MAX_IDENTIFIER_SIZE: usize = 2050;

/// The resource that is to be created or updated on Sui.
///
/// This struct contains additional information that is not stored on chain, compared to
/// [`SuiResource`] (`unencoded_size`, `full_path`).
///
/// [`Resource`] objects are always compared on their `info` field
/// ([`SuiResource`]), and never on their `unencoded_size` or `full_path`.
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Resource {
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
    pub const QUILT_PATCH_ID_INTERNAL_HEADER: &str = "x-wal-quilt-patch-internal-id";

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

    /// Returns the quilt patch ID from the resource's internal headers, if present.
    pub fn patch_id(&self) -> Option<&String> {
        self.info.headers.get(Self::QUILT_PATCH_ID_INTERNAL_HEADER)
    }

    /// Creates a [`Resource`] from local file data and blob storage information.
    ///
    /// Used when an unchanged file is reused from an existing site, preserving its
    /// blob ID and quilt patch ID rather than re-uploading.
    pub fn from_resource_data(
        ResourceData {
            unencoded_size,
            full_path,
            resource_path,
            mut headers,
            blob_hash,
        }: ResourceData,
        blob_id: BlobId,
        x_wal_quilt_patch_id: Option<String>,
    ) -> Self {
        if let Some(patch_id) = x_wal_quilt_patch_id {
            headers
                .0
                .insert(Self::QUILT_PATCH_ID_INTERNAL_HEADER.to_owned(), patch_id);
        };

        Resource {
            info: SuiResource {
                path: resource_path,
                headers,
                blob_id,
                blob_hash,
                range: None,
            },
            unencoded_size,
            full_path,
        }
    }
}

/// Converts a [`Resource`] back to [`ResourceData`] for re-storage.
///
/// Used when a resource's blob has expired and needs to be stored again.
/// The quilt patch ID header is removed since a new one will be assigned.
impl From<Resource> for ResourceData {
    fn from(resource: Resource) -> Self {
        let mut headers = resource.info.headers;
        headers
            .0
             .0
            .remove(Resource::QUILT_PATCH_ID_INTERNAL_HEADER);

        ResourceData {
            unencoded_size: resource.unencoded_size,
            full_path: resource.full_path,
            resource_path: resource.info.path,
            headers,
            blob_hash: resource.info.blob_hash,
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
#[derive(Clone)]
pub enum SiteOps<'a> {
    Deleted(&'a Resource),
    Created(&'a Resource),
    Unchanged(&'a Resource),
    RemovedRoutes,
    BurnedSite,
}

impl fmt::Debug for SiteOps<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (op, path) = match self {
            SiteOps::Deleted(resource) => ("delete", &resource.info.path),
            SiteOps::Created(resource) => ("create", &resource.info.path),
            SiteOps::Unchanged(resource) => ("unchanged", &resource.info.path),
            SiteOps::RemovedRoutes => ("remove routes", &"".to_string()),
            SiteOps::BurnedSite => ("burn site", &"".to_string()),
        };
        f.debug_struct("ResourceOp")
            .field("operation", &op)
            .field("path", path)
            .finish()
    }
}

impl<'a> SiteOps<'a> {
    /// Returns the resource for which this operation is defined.
    pub fn resource(&self) -> Option<&'a Resource> {
        match self {
            SiteOps::Deleted(resource) => Some(resource),
            SiteOps::Created(resource) => Some(resource),
            SiteOps::Unchanged(resource) => Some(resource),
            SiteOps::RemovedRoutes => None,
            SiteOps::BurnedSite => None,
        }
    }

    /// Returns if the operation needs to be uploaded to Walrus.
    pub fn is_walrus_update(&self) -> bool {
        matches!(self, SiteOps::Created(_))
    }

    /// Returns true if the operation modifies a resource.
    pub fn is_change(&self) -> bool {
        matches!(self, SiteOps::Created(_) | SiteOps::Deleted(_))
    }
}

/// A set of resources composing a site.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct ResourceSet {
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
    pub fn diff<'a>(&'a self, start: &'a ResourceSet) -> Vec<SiteOps<'a>> {
        let create = self.inner.difference(&start.inner).map(SiteOps::Created);
        let delete = start.inner.difference(&self.inner).map(SiteOps::Deleted);
        let unchanged = self
            .inner
            .intersection(&start.inner)
            .map(SiteOps::Unchanged);
        delete.chain(create).chain(unchanged).collect()
    }

    /// Returns a vector of operations to delete all resources in the set.
    pub fn delete_all(&self) -> Vec<SiteOps<'_>> {
        self.inner.iter().map(SiteOps::Deleted).collect()
    }

    /// Returns a vector of operations to create all resources in the set.
    pub fn create_all(&self) -> Vec<SiteOps<'_>> {
        self.inner.iter().map(SiteOps::Created).collect()
    }

    /// Returns a vector of operations to replace the resources in `self` with the ones in `other`.
    pub fn replace_all<'a>(&'a self, other: &'a ResourceSet) -> Vec<SiteOps<'a>> {
        // Delete all the resources already on chain.
        let mut delete_operations = self.delete_all();
        // Create all the resources on disk.
        let create_operations = other.create_all();
        delete_operations.extend(create_operations);
        delete_operations
    }

    /// Extends the set with the resources in the iterator.
    pub fn extend<I, R>(&mut self, resources: I)
    where
        I: IntoIterator<Item = R>,
        R: Into<Resource>,
    {
        self.inner.extend(resources.into_iter().map(Into::into));
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub(crate) fn len(&self) -> usize {
        self.inner.len()
    }

    pub(crate) fn iter(&self) -> std::collections::btree_set::Iter<'_, Resource> {
        self.inner.iter()
    }
}

impl<'a> IntoIterator for &'a ResourceSet {
    type Item = &'a Resource;
    type IntoIter = std::collections::btree_set::Iter<'a, Resource>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
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

/// Local file data for a resource before it is uploaded to Walrus.
///
/// Contains the file's metadata (path, size, headers, hash) but not the blob ID,
/// which is only assigned after upload.
#[derive(Debug, Clone)]
pub struct ResourceData {
    unencoded_size: usize,
    full_path: PathBuf,
    pub resource_path: String,
    headers: HttpHeaders,
    pub blob_hash: U256,
}

impl ResourceData {
    /// Creates a deterministic mock blob ID for estimation purposes.
    pub fn create_mock_blob_id(&self) -> BlobId {
        use fastcrypto::hash::HashFunction;

        // Create a deterministic hash based on resource path and content hash
        let mut hasher = fastcrypto::hash::Sha256::default();
        hasher.update(self.resource_path.as_bytes());
        hasher.update(self.blob_hash.to_le_bytes());
        let hash = hasher.finalize();

        // Convert to first 32 bytes to a BlobId
        let blob_id_bytes: [u8; 32] = hash.as_ref()[..32].try_into().unwrap();
        BlobId::try_from(&blob_id_bytes[..]).expect("Invalid blob ID length")
    }

    /// Converts this ResourceData into a mock Resource for estimation purposes.
    pub fn into_mock_resource(self) -> Resource {
        let mock_blob_id = self.create_mock_blob_id();
        let mock_patch_id = Some(format!("0x{}", hex::encode([0u8; 32]))); // Mock patch ID
        self.into_resource(mock_blob_id, mock_patch_id)
    }
}

/// The result of parsing a directory for resources.
///
/// Contains resources split into unchanged (can reuse existing blob IDs) and
/// changed (need new storage).
#[derive(Debug)]
pub struct ParsedResources {
    /// Resources that haven't changed and can reuse their existing blob IDs.
    pub unchanged: Vec<Resource>,
    /// Resources that have changed and need to be stored into new quilts.
    pub changed: Vec<ResourceData>,
}

impl ResourceData {
    /// Reads a local file and creates [`ResourceData`] from it.
    ///
    /// Returns `None` if the file should be ignored (matches ignore patterns or is
    /// the ws-resources.json config file itself).
    pub fn from_file(
        ws_resources_path: Option<&Path>,
        ws_resources: Option<&WSResources>,
        full_path: &Path,
        resource_path: String,
    ) -> anyhow::Result<Option<ResourceData>> {
        // Validate identifier size (BCS serialized) should be just 1 + str.len(), but
        // this is cleaner
        let identifier_size =
            bcs::serialized_size(&resource_path).context("Failed to compute identifier size")?;
        if identifier_size > MAX_IDENTIFIER_SIZE {
            bail!(
                "Identifier for '{resource_path}' is too long: {identifier_size} bytes (max: {MAX_IDENTIFIER_SIZE} bytes). \
                Consider using a shorter path name.",
            );
        }

        if let Some(ws_path) = ws_resources_path {
            if full_path == ws_path {
                tracing::debug!(?full_path, "ignoring the ws-resources config file");
                return Ok(None);
            }
        }

        // Skip if resource matches ignore patterns/
        if let Some(ws_resources) = ws_resources {
            let mut ignore_iter = ws_resources
                .ignore
                .as_deref()
                .into_iter()
                .flatten()
                .map(String::as_str);
            if is_ignored(&mut ignore_iter, &resource_path) {
                tracing::debug!(?resource_path, "ignoring resource due to ignore pattern");
                return Ok(None);
            }
        }

        let mut http_headers: VecMap<String, String> =
            Self::derive_http_headers(ws_resources, &resource_path);
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

        // Hash the contents of the file - this will be contained in the site::Resource
        // to verify the integrity of the blob when fetched from an aggregator.
        let mut hash_function = Sha256::default();
        hash_function.update(&plain_content);
        let blob_hash: [u8; 32] = hash_function.finalize().digest;

        Ok(Some(ResourceData {
            unencoded_size: plain_content.len(),
            resource_path,
            full_path: full_path.to_owned(),
            headers: HttpHeaders(http_headers),
            blob_hash: U256::from_le_bytes(&blob_hash),
        }))
    }

    ///  Derives the HTTP headers for a resource based on the ws-resources.yaml.
    ///
    ///  Matches the path of the resource to the wildcard paths in the configuration to
    ///  determine the headers to be added to the HTTP response.
    fn derive_http_headers(
        ws_resources: Option<&WSResources>,
        resource_path: &str,
    ) -> VecMap<String, String> {
        ws_resources
            .and_then(|config| config.headers.as_ref())
            .and_then(|headers| {
                headers
                    .iter()
                    .filter(|(path, _)| is_pattern_match(path, resource_path))
                    .max_by_key(|(path, _)| path.split('/').count())
                    .map(|(_, header_map)| header_map.0.clone())
            })
            .unwrap_or_default()
    }

    /// Returns the unencoded size of the resource.
    pub fn unencoded_size(&self) -> usize {
        self.unencoded_size
    }

    /// Returns the full path of the resource on disk.
    pub fn full_path(&self) -> &Path {
        &self.full_path
    }

    /// Converts this [`ResourceData`] into a [`Resource`] with the given blob ID and patch ID.
    pub fn into_resource(self, blob_id: BlobId, patch_hex: Option<String>) -> Resource {
        let mut headers = self.headers;
        if let Some(patch) = patch_hex {
            headers
                .0
                .insert(Resource::QUILT_PATCH_ID_INTERNAL_HEADER.to_string(), patch);
        }
        Resource::new(
            self.resource_path,
            self.full_path,
            headers,
            blob_id,
            self.blob_hash,
            self.unencoded_size,
        )
    }
}

/// Converts [`ResourceData`] to a [`QuiltBlobInput`] for the Walrus CLI.
impl From<&ResourceData> for QuiltBlobInput {
    fn from(value: &ResourceData) -> QuiltBlobInput {
        QuiltBlobInput {
            path: value.full_path.clone(),
            identifier: Some(value.resource_path.clone()),
            tags: BTreeMap::new(),
        }
    }
}

/// Loads and manages the set of resources composing the site.
#[derive(Debug)]
pub(crate) struct ResourceManager {
    /// The ws-resources.json contents.
    pub ws_resources: Option<WSResources>,
    /// The ws-resource file path.
    pub ws_resources_path: Option<PathBuf>,
}

impl ResourceManager {
    pub fn new(
        ws_resources: Option<WSResources>,
        ws_resources_path: Option<PathBuf>,
    ) -> Result<Self> {
        // Cast the keys to lowercase because http headers
        //  are case-insensitive: RFC7230 sec. 2.7.3
        if let Some(resources) = ws_resources.as_ref() {
            if let Some(ref headers) = resources.headers {
                for (_, header_map) in headers.clone().iter_mut() {
                    header_map.0 = header_map
                        .0
                        .iter()
                        .map(|(k, v)| (k.to_lowercase(), v.clone()))
                        .collect();
                }
            }
        }

        Ok(ResourceManager {
            ws_resources,
            ws_resources_path,
        })
    }

    /// Parse resource paths into resource data for storage.
    ///
    /// This is used by the `update-resources` command to parse individual files.
    pub fn parse_resources(&self, resource_args: Vec<ResourcePaths>) -> Result<Vec<ResourceData>> {
        let resource_data = resource_args
            .into_iter()
            .map(
                |ResourcePaths {
                     file_path,
                     url_path,
                 }| {
                    ResourceData::from_file(
                        self.ws_resources_path.as_deref(),
                        self.ws_resources.as_ref(),
                        file_path.as_path(),
                        url_path,
                    )
                },
            )
            .collect::<Result<Vec<Option<_>>>>()?
            .into_iter()
            .flatten()
            .collect();
        Ok(resource_data)
    }

    /// Recursively iterate a directory and parse all resources within.
    ///
    /// Returns a [`ParsedResources`] containing unchanged resources (can reuse existing blob IDs)
    /// and changed resources (need new storage).
    pub fn read_dir(&self, root: &Path, existing_site: &SiteData) -> Result<ParsedResources> {
        let resource_paths = Self::iter_dir(root)?;
        if resource_paths.is_empty() {
            return Ok(ParsedResources {
                unchanged: vec![],
                changed: vec![],
            });
        }

        let rel_paths = resource_paths
            .into_iter()
            .map(|file_path| {
                let url_path = full_path_to_resource_path(file_path.as_path(), root)?;
                Ok(ResourcePaths {
                    file_path,
                    url_path,
                })
            })
            .collect::<Result<Vec<ResourcePaths>>>()?;

        let local_resources = rel_paths
            .into_iter()
            .map(
                |ResourcePaths {
                     file_path,
                     url_path,
                 }| {
                    ResourceData::from_file(
                        self.ws_resources_path.as_deref(),
                        self.ws_resources.as_ref(),
                        file_path.as_path(),
                        url_path,
                    )
                },
            )
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten();

        let (changed, unchanged) =
            local_resources.fold((vec![], vec![]), |(mut changed, mut unchanged), local| {
                let site_resource = existing_site
                    .resources()
                    .inner
                    .iter()
                    .find(|res| res.info.path == local.resource_path);

                match site_resource {
                    Some(res) if res.info.blob_hash == local.blob_hash => {
                        // Even if data is the same, other things might have changed.
                        let updated_resource = Resource::from_resource_data(
                            local,
                            res.info.blob_id,
                            res.patch_id().cloned(),
                        );
                        unchanged.push(updated_resource);
                    }
                    _ => {
                        changed.push(local);
                    }
                }

                (changed, unchanged)
            });

        // Warn about unchanged resources stored as legacy blobs (without quilt patch IDs).
        let legacy_blob_count = unchanged.iter().filter(|r| r.patch_id().is_none()).count();
        if legacy_blob_count > 0 {
            display::warning(format!(
                "Found {legacy_blob_count} resource(s) stored as individual blobs (legacy format). \
                To benefit from quilt optimizations, re-publish the site or use `update-resources` \
                to migrate specific files."
            ));
        }

        Ok(ParsedResources { unchanged, changed })
    }

    fn iter_dir(start: &Path) -> Result<Vec<PathBuf>> {
        let mut resources = vec![];
        let entries = fs::read_dir(start)?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                resources.extend(Self::iter_dir(&path)?);
            } else {
                resources.push(path.to_owned());
            }
        }
        Ok(resources)
    }

    pub fn to_site_data(&self, set: ResourceSet) -> SiteData {
        SiteData::new(
            set,
            self.ws_resources
                .as_ref()
                .and_then(|config| config.routes.clone()),
            self.ws_resources
                .as_ref()
                .and_then(|config| config.metadata.clone()),
            self.ws_resources
                .as_ref()
                .and_then(|config| config.site_name.clone()),
        )
    }

    /// Creates mock resources from chunked resource data for estimation purposes.
    pub fn create_mock_resources_from_chunks(
        &self,
        chunks: &[Vec<(ResourceData, crate::walrus::command::QuiltBlobInput)>],
    ) -> Vec<Resource> {
        let mut mock_resources = Vec::new();
        for chunk in chunks {
            for (res_data, _) in chunk {
                mock_resources.push(res_data.clone().into_mock_resource());
            }
        }
        mock_resources
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
    let path_str = rel_path
        .to_str()
        .ok_or(anyhow!("could not process the path string: {rel_path:?}"))?;

    // Normalize Windows path separators to URL-style forward slashes.
    // Only needed on Windows; on Unix, backslash can be a valid filename character.
    #[cfg(windows)]
    let path_str = path_str.replace('\\', "/");
    Ok(format!("/{}", path_str))
}

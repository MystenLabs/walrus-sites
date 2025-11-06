// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Functionality to read and check the files in of a website.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::{self, Debug, Display},
    fs,
    io::Write,
    num::{NonZeroU16, NonZeroUsize},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use bytesize::ByteSize;
use fastcrypto::hash::{HashFunction, Sha256};
use flate2::{write::GzEncoder, Compression};
use futures::{stream, StreamExt};
use itertools::Itertools;
use move_core_types::u256::U256;
use tracing::debug;

use super::SiteData;
use crate::{
    args::EpochArg,
    display,
    publish::BlobManagementOptions,
    site::{config::WSResources, content::ContentType},
    types::{HttpHeaders, SuiResource, VecMap},
    util::{is_ignored, is_pattern_match},
    walrus::{
        command::{QuiltBlobInput, StoreQuiltInput},
        types::BlobId,
        Walrus,
    },
};

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
pub enum ResourceOp<'a> {
    Deleted(&'a Resource),
    Created(&'a Resource),
    Unchanged(&'a Resource),
}

impl fmt::Debug for ResourceOp<'_> {
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
    pub fn is_walrus_update(&self, blob_options: &BlobManagementOptions) -> bool {
        matches!(self, ResourceOp::Created(_))
            || (blob_options.is_check_extend() && matches!(self, ResourceOp::Unchanged(_)))
    }

    /// Returns true if the operation modifies a resource.
    pub fn is_change(&self) -> bool {
        matches!(self, ResourceOp::Created(_) | ResourceOp::Deleted(_))
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
    pub fn delete_all(&self) -> Vec<ResourceOp<'_>> {
        self.inner.iter().map(ResourceOp::Deleted).collect()
    }

    /// Returns a vector of operations to create all resources in the set.
    pub fn create_all(&self) -> Vec<ResourceOp<'_>> {
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

// Struct used for grouping resource local data.
#[derive(Debug)]
struct ResourceData {
    unencoded_size: usize,
    full_path: PathBuf,
    resource_path: String,
    headers: HttpHeaders,
    blob_hash: U256,
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
    /// The maximum number of concurrent calls to the walrus cli for computing the blob ID.
    pub max_concurrent: Option<NonZeroUsize>,
}

impl ResourceManager {
    pub async fn new(
        walrus: Walrus,
        ws_resources: Option<WSResources>,
        ws_resources_path: Option<PathBuf>,
        max_concurrent: Option<NonZeroUsize>,
    ) -> Result<Self> {
        let n_shards = walrus.n_shards().await?;

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
            walrus,
            ws_resources,
            ws_resources_path,
            n_shards,
            max_concurrent,
        })
    }

    fn read_local_resource(
        &self,
        full_path: &Path,
        resource_path: String,
    ) -> Result<Option<ResourceData>> {
        if let Some(ws_path) = &self.ws_resources_path {
            if full_path == ws_path {
                tracing::debug!(?full_path, "ignoring the ws-resources config file");
                return Ok(None);
            }
        }

        // Skip if resource matches ignore patterns/
        if let Some(ws_resources) = &self.ws_resources {
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
            ResourceManager::derive_http_headers(&self.ws_resources, &resource_path);
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

    /// Read a resource at a path.
    ///
    /// Ignores empty files.
    pub async fn read_single_blob_resource(
        &self,
        full_path: &Path,
        resource_path: String,
    ) -> Result<Option<Resource>> {
        let Some(ResourceData {
            unencoded_size,
            full_path,
            resource_path,
            headers,
            blob_hash,
        }) = self.read_local_resource(full_path, resource_path)?
        else {
            return Ok(None);
        };

        let output = self
            .walrus
            .blob_id(full_path.to_owned(), Some(self.n_shards))
            .await
            .context(format!(
                "error while computing the blob id for path: {}",
                full_path.to_string_lossy()
            ))?;

        Ok(Some(Resource::new(
            resource_path,
            full_path,
            headers,
            output.blob_id,
            blob_hash,
            unencoded_size,
        )))
    }

    ///  Derives the HTTP headers for a resource based on the ws-resources.yaml.
    ///
    ///  Matches the path of the resource to the wildcard paths in the configuration to
    ///  determine the headers to be added to the HTTP response.
    pub fn derive_http_headers(
        ws_resources: &Option<WSResources>,
        resource_path: &str,
    ) -> VecMap<String, String> {
        ws_resources
            .as_ref()
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

    /// Filters resource_paths, and prepares their ResourceData, while also grouping them into a
    /// single Quilt.
    fn prepare_local_resources(
        &mut self,
        full_path: &Path,
        resource_paths: Vec<String>,
    ) -> Result<Vec<(ResourceData, QuiltBlobInput)>> {
        let res = resource_paths
            .into_iter()
            .map(|resource_path| {
                let full_path = full_path.join(
                    resource_path
                        .strip_prefix('/')
                        .unwrap_or(resource_path.as_str()),
                );

                // Validate identifier size (BCS serialized) should be just 1 + str.len(), but
                // this is cleaner
                let identifier_size = bcs::serialized_size(&resource_path)
                    .context("Failed to compute identifier size")?;
                if identifier_size > MAX_IDENTIFIER_SIZE {
                    bail!(
                        "Identifier for '{resource_path}' is too long: {identifier_size} bytes (max: {MAX_IDENTIFIER_SIZE} bytes). \
                        Consider using a shorter path name.",
                    );
                }

                let Some(res_data) = self.read_local_resource(&full_path, resource_path.clone())?
                else {
                    return Ok(None);
                };
                let quilt_blob_input = QuiltBlobInput {
                    path: full_path.clone(),
                    identifier: Some(resource_path),
                    tags: BTreeMap::new(),
                };
                Ok(Some((res_data, quilt_blob_input)))
            })
            .collect::<Result<Vec<Option<(ResourceData, QuiltBlobInput)>>>>()?
            .into_iter()
            .flatten()
            .collect();

        Ok(res)
    }

    // Extracts Blob-id and Quilt-patch-id from the walrus store response, in order to combine
    // it with ResourceData to return Resources
    async fn store_resource_chunk_into_quilt(
        &mut self,
        res_data_and_quilt_files: impl IntoIterator<Item = (ResourceData, QuiltBlobInput)>,
        epochs: EpochArg,
    ) -> Result<Vec<Resource>> {
        // println!("resource_data.len(): {}", resource_data.len());
        // println!("resource_data: {resource_data:#?}");

        let (resource_data, quilt_blob_inputs): (Vec<_>, Vec<_>) =
            res_data_and_quilt_files.into_iter().unzip();
        let mut store_resp = self
            .walrus
            .store_quilt(
                StoreQuiltInput::Blobs(quilt_blob_inputs.clone()),
                epochs,
                false,
                true,
            )
            .await
            .context(format!(
                "error while storing the quilt for resources: {}",
                quilt_blob_inputs
                    .iter()
                    .map(|f| f.path.display())
                    .join(", ")
            ))?;

        debug!(
            "quilt store output: {}",
            serde_json::to_string_pretty(&store_resp)?
        );

        let blob_id = *store_resp.blob_store_result.blob_id();
        let quilt_patches = &mut store_resp.stored_quilt_blobs;

        resource_data
            .into_iter()
            .map(
                |
                    ResourceData {
                        unencoded_size,
                        full_path,
                        resource_path,
                        mut headers,
                        blob_hash,
                    }
                | {
                    // TODO(nikos): We have already calculated this
                    let patch_identifier = resource_path.as_str();
                    // Walrus store does not maintain the order the files were passed
                    let Some(pos) = quilt_patches.iter().position(|p| p.identifier == patch_identifier) else {
                        bail!("Resource {resource_path} exists but doesn't have a matching quilt-patch");
                    };
                    let patch = quilt_patches.swap_remove(pos);
                    let bytes = URL_SAFE_NO_PAD.decode(patch.quilt_patch_id.as_str())?;

                    // Must have at least BlobId.LENGTH + 1 bytes (quilt_id + version).
                    if bytes.len() != Walrus::QUILT_PATCH_ID_SIZE {
                        bail!(
                            "Expected {} bytes when decoding quilt-patch-id version 1.",
                            Walrus::QUILT_PATCH_ID_SIZE
                        );
                    }

                    // Extract patch_id (bytes after the blob_id).
                    let patch_bytes: [u8; Walrus::QUILT_PATCH_SIZE] = bytes
                        [BlobId::LENGTH..Walrus::QUILT_PATCH_ID_SIZE]
                        .try_into()
                        .unwrap();
                    let version = patch_bytes[0];
                    if version != Walrus::QUILT_PATCH_VERSION_1 {
                        bail!("Quilt patch version {version} is not implemented");
                    }

                    let patch_hex = format!("0x{}", hex::encode(patch_bytes));
                    headers
                        .0
                        .insert(Resource::QUILT_PATCH_ID_INTERNAL_HEADER.to_string(), patch_hex);
                    Ok(Resource::new(
                        resource_path,
                        full_path,
                        headers,
                        blob_id,
                        blob_hash,
                        unencoded_size,
                    ))
                },
            )
            .collect::<Result<Vec<_>>>()
    }

    async fn dry_run_resource_chunk(
        &mut self,
        quilt_blob_inputs: Vec<QuiltBlobInput>,
        epochs: EpochArg,
    ) -> Result<u64> {
        let store_resp = self
            .walrus
            .dry_run_store_quilt(
                StoreQuiltInput::Blobs(quilt_blob_inputs.clone()),
                epochs,
                false,
                true,
            )
            .await
            .context(format!(
                "error while computing the blob id for resources in path: {}",
                quilt_blob_inputs
                    .iter()
                    .map(|f| f.path.display())
                    .join(", ")
            ))?;
        Ok(store_resp.quilt_blob_output.storage_cost)
    }

    /// Recursively iterate a directory and load all [`Resources`][Resource] within.
    pub async fn read_dir(&mut self, root: &Path) -> Result<SiteData> {
        let resource_paths = Self::iter_dir(root)?;
        if resource_paths.is_empty() {
            return Ok(SiteData::empty());
        }

        let futures = resource_paths
            .iter()
            .map(|full_path| {
                full_path_to_resource_path(full_path, root)
                    .map(|resource_path| self.read_single_blob_resource(full_path, resource_path))
            })
            .collect::<Result<Vec<_>>>()?;

        // Limit the amount of futures awaited concurrently.
        let concurrency_limit = self
            .max_concurrent
            .map(NonZeroUsize::get)
            .unwrap_or_else(|| resource_paths.len());

        let mut stream = stream::iter(futures).buffer_unordered(concurrency_limit);

        let mut resources = ResourceSet::empty();
        while let Some(resource) = stream.next().await {
            resources.extend(resource?);
        }

        Ok(self.to_site_data(resources))
    }

    /// Recursively iterate a directory and load all [`Resources`][Resource] within.
    pub async fn read_dir_and_store_quilts(
        &mut self,
        root: &Path,
        epochs: EpochArg,
        dry_run: bool,
        max_quilt_size: ByteSize,
    ) -> Result<SiteData> {
        let resource_paths = Self::iter_dir(root)?;
        if resource_paths.is_empty() {
            return Ok(SiteData::empty());
        }

        let rel_paths = resource_paths
            .iter()
            .map(|full_path| full_path_to_resource_path(full_path, root))
            .collect::<Result<Vec<String>>>()?;

        let resource_file_inputs = self.prepare_local_resources(root, rel_paths)?;
        let chunks = self.quilts_chunkify(resource_file_inputs, max_quilt_size)?;

        if dry_run {
            let mut total_storage_cost = 0;
            for chunk in &chunks {
                let quilt_file_inputs = chunk.iter().map(|(_, f)| f.clone()).collect_vec();
                let wal_storage_cost = self
                    .dry_run_resource_chunk(quilt_file_inputs, epochs.clone())
                    .await?;
                total_storage_cost += wal_storage_cost;
            }

            display::action(format!(
                    "Estimated Storage Cost for this publish/update (Gas Cost Excluded): {total_storage_cost} FROST"
                ));

            // Add user confirmation prompt.
            #[cfg(test)]
            display::action("Waiting for user confirmation...");
            #[cfg(not(feature = "_testing-dry-run"))]
            {
                if !dialoguer::Confirm::new()
                    .with_prompt("Do you want to proceed with these updates?")
                    .default(true)
                    .interact()?
                {
                    display::error("Update cancelled by user");
                    return Err(anyhow!("Update cancelled by user"));
                }
            }
            #[cfg(feature = "_testing-dry-run")]
            {
                // In tests, automatically proceed without prompting
                println!("Test mode: automatically proceeding with updates");
            }
        }

        let mut resources_set = ResourceSet::empty();
        tracing::debug!("Processing chunks for quilt storage");

        for chunk in chunks {
            let resources = self
                .store_resource_chunk_into_quilt(chunk, epochs.clone())
                .await?;
            resources_set.extend(resources);
        }

        tracing::debug!(
            "Final site data will be created with {} resources",
            resources_set.len()
        );
        Ok(self.to_site_data(resources_set))
    }

    fn quilts_chunkify(
        &self,
        resources: Vec<(ResourceData, QuiltBlobInput)>,
        max_quilt_size: ByteSize,
    ) -> Result<Vec<Vec<(ResourceData, QuiltBlobInput)>>> {
        let max_size_per_quilt = max_quilt_size.as_u64() as usize;
        let mut chunks = vec![];
        let mut current_chunk = vec![];

        // Available columns: n_cols - 1 (reserve 1 for quilt index)
        let max_available_columns = Walrus::max_slots_in_quilt(self.n_shards) as usize;
        let mut available_columns = max_available_columns;
        // Calculate capacity per column (slot) in bytes
        let column_capacity = max_size_per_quilt / available_columns;

        // Per-file overhead constant
        const FIXED_OVERHEAD: usize = 8; // BLOB_IDENTIFIER_SIZE_BYTES_LENGTH (2) + BLOB_HEADER_SIZE (6)

        for (res_data, quilt_input) in resources.into_iter() {
            // Calculate total size including overhead
            let file_size_with_overhead =
                res_data.unencoded_size + MAX_IDENTIFIER_SIZE + FIXED_OVERHEAD;

            // Abort if the file cannot fit in a single Quilt.
            // TODO(fix): We could still store a single-file quilt for this case.
            if file_size_with_overhead > max_size_per_quilt {
                bail!(
                    "File size of {} exceeds maximum size of single file storage in Quilt.",
                    res_data.full_path.as_path().display()
                );
            }

            // Calculate how many columns this file needs
            let columns_needed = file_size_with_overhead.div_ceil(column_capacity);

            if available_columns >= columns_needed {
                // File fits in current chunk
                current_chunk.push((res_data, quilt_input));
                available_columns -= columns_needed;
            } else {
                // File doesn't fit, start a new chunk
                if !current_chunk.is_empty() {
                    chunks.push(current_chunk);
                }
                current_chunk = vec![(res_data, quilt_input)];
                // Reset available columns for new chunk
                available_columns = max_available_columns - columns_needed;
            }
        }

        // Push the last chunk if it's not empty
        if !current_chunk.is_empty() {
            chunks.push(current_chunk);
        }

        Ok(chunks)
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

    fn to_site_data(&self, set: ResourceSet) -> SiteData {
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
            .ok_or(anyhow!("could not process the path string: {rel_path:?}"))?
    ))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{HttpHeaders, ResourceManager};
    use crate::site::config::WSResources;

    #[test]
    fn test_derive_http_headers() {
        let test_paths = vec![
            // This is the longest path. So `/foo/bar/baz/*.svg` would persist over `*.svg`.
            ("/foo/bar/baz/image.svg", "etag"),
            // This will only match `*.svg`.
            (
                "/very_long_name_that_should_not_be_matched.svg",
                "cache-control",
            ),
        ];
        let ws_resources = mock_ws_resources();
        for (path, expected) in test_paths {
            let result = ResourceManager::derive_http_headers(&ws_resources, path);
            assert_eq!(result.len(), 1);
            assert!(result.contains_key(expected));
        }
    }

    /// Helper function for testing the `derive_http_headers` method.
    fn mock_ws_resources() -> Option<WSResources> {
        let headers_json = r#"{
                    "/*.svg": {
                        "cache-control": "public, max-age=86400"
                    },
                    "/foo/bar/baz/*.svg": {
                        "etag": "\"abc123\""
                    }
                }"#;
        let headers: BTreeMap<String, HttpHeaders> = serde_json::from_str(headers_json).unwrap();

        Some(WSResources {
            headers: Some(headers),
            routes: None,
            metadata: None,
            site_name: None,
            object_id: None,
            ignore: None,
        })
    }
}

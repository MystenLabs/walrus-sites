// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Functionality to read and check the files in of a website.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::{self, Debug, Display},
    fs,
    io::Write,
    num::NonZeroU16,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use bytesize::ByteSize;
use fastcrypto::hash::{HashFunction, Sha256};
use flate2::{write::GzEncoder, Compression};
use itertools::Itertools;
use move_core_types::u256::U256;

use super::SiteData;
use crate::{
    args::{EpochArg, ResourcePaths},
    display,
    site::{config::WSResources, content::ContentType},
    types::{HttpHeaders, SuiResource, VecMap},
    util::{is_ignored, is_pattern_match},
    walrus::{
        command::{QuiltBlobInput, StoreQuiltInput},
        types::BlobId,
        Walrus,
    },
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
#[derive(Debug)]
pub struct ResourceData {
    unencoded_size: usize,
    full_path: PathBuf,
    resource_path: String,
    headers: HttpHeaders,
    blob_hash: U256,
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

    /*
    pub fn try_into_resource(self, patch: StoredQuiltPatch) -> anyhow::Result<Resource> {
        let ResourceData {
            unencoded_size,
            full_path,
            resource_path,
            mut headers,
            blob_hash,
        } = self;
        let bytes = BASE64_URL_SAFE_NO_PAD.decode(patch.quilt_patch_id.as_str())?;

        // Must have at least BlobId.LENGTH + 1 bytes (quilt_id + version).
        if bytes.len() != Walrus::QUILT_PATCH_ID_SIZE {
            anyhow::bail!(
                "Expected {} bytes when decoding quilt-patch-id version 1.",
                Walrus::QUILT_PATCH_ID_SIZE
            );
        }

        // Extract blob_id and patch_id (bytes after the blob_id).
        let (blob_id, patch_bytes): (BlobId, [u8; Walrus::QUILT_PATCH_SIZE]) = (
            BlobId(bytes[..BlobId::LENGTH].try_into().unwrap()),
            bytes[BlobId::LENGTH..Walrus::QUILT_PATCH_ID_SIZE]
                .try_into()
                .unwrap(),
        );

        let version = patch_bytes[0];
        if version != Walrus::QUILT_PATCH_VERSION_1 {
            anyhow::bail!("Quilt patch version {version} is not implemented");
        }

        let patch_hex = format!("0x{}", hex::encode(patch_bytes));
        headers.0.insert(
            Resource::QUILT_PATCH_ID_INTERNAL_HEADER.to_string(),
            patch_hex,
        );
        Ok(Resource::new(
            resource_path,
            full_path,
            headers,
            blob_id,
            blob_hash,
            unencoded_size,
        ))
    }
    */
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
        })
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

    pub async fn parse_resources_and_store_quilts(
        &mut self,
        resource_args: Vec<ResourcePaths>,
        epochs: EpochArg,
        dry_run: bool,
        max_quilt_size: ByteSize,
    ) -> Result<ResourceSet> {
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

        let resources_set = self
            .store_into_quilts(resource_data, epochs, dry_run, max_quilt_size)
            .await?;
        tracing::debug!(
            "Final site data will be created with {} resources",
            resources_set.len()
        );
        Ok(resources_set)
    }

    /// Recursively iterate a directory and load all [`Resources`][Resource] within.
    pub async fn read_dir_and_store_quilts(
        &mut self,
        root: &Path,
        epochs: EpochArg,
        dry_run: bool,
        max_quilt_size: ByteSize,
        existing_site: SiteData,
    ) -> Result<SiteData> {
        let resource_paths = Self::iter_dir(root)?;
        if resource_paths.is_empty() {
            return Ok(SiteData::empty());
        }

        // TODO(sew-495): Remove the multiple collects
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
            .flatten()
            .collect::<Vec<_>>();

        // TODO(sew-495): Extend old Quilts
        // TODO(sew-495): What about files stored as blobs? It doesn't make much business
        // sense to keep them as blobs.
        let (file_changed, file_unchanged) = local_resources.into_iter().fold(
            (vec![], vec![]),
            |(mut changed, mut unchanged), local| {
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
            },
        );

        // changed files need to be stored into new quilts
        let mut resources_set = self
            .store_into_quilts(file_changed, epochs, dry_run, max_quilt_size)
            .await?;
        resources_set.extend(file_unchanged.into_iter());

        tracing::debug!(
            "Final site data will be created with {} resources",
            resources_set.len()
        );
        Ok(self.to_site_data(resources_set))
    }

    // TODO: Accept iterator?
    async fn store_into_quilts(
        &mut self,
        resource_file_inputs: Vec<ResourceData>,
        epochs: EpochArg,
        dry_run: bool,
        max_quilt_size: ByteSize,
    ) -> anyhow::Result<ResourceSet> {
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
        Ok(resources_set)
    }

    fn quilts_chunkify(
        &self,
        resources: Vec<ResourceData>,
        max_quilt_size: ByteSize,
    ) -> Result<Vec<Vec<(ResourceData, QuiltBlobInput)>>> {
        let max_quilt_size = max_quilt_size.as_u64() as usize;
        let max_available_columns = Walrus::max_slots_in_quilt(self.n_shards) as usize;
        let max_theoretical_quilt_size =
            Walrus::max_slot_size(self.n_shards) * max_available_columns;

        // Cap the effective_quilt_size to the min between the theoretical and the passed
        let effective_quilt_size = if max_theoretical_quilt_size < max_quilt_size {
            display::warning(format!(
                "Configured max quilt size ({}) exceeds theoretical maximum ({}). Using {} instead.",
                ByteSize(max_quilt_size as u64),
                ByteSize(max_theoretical_quilt_size as u64),
                ByteSize(max_theoretical_quilt_size as u64)
            ));
            max_theoretical_quilt_size
        } else {
            max_quilt_size
        };
        let mut available_columns = max_available_columns;
        // Calculate capacity per column (slot) in bytes
        let column_capacity = effective_quilt_size / available_columns;
        // Per-file overhead constant
        const FIXED_OVERHEAD: usize = 8; // BLOB_IDENTIFIER_SIZE_BYTES_LENGTH (2) + BLOB_HEADER_SIZE (6)

        let mut chunks = vec![];
        let mut current_chunk = vec![];

        for res_data in resources.into_iter() {
            let quilt_input = (&res_data).into();
            // Calculate total size including overhead
            let file_size_with_overhead =
                res_data.unencoded_size + MAX_IDENTIFIER_SIZE + FIXED_OVERHEAD;

            // Abort if the file cannot fit even in the theoretical maximum
            if file_size_with_overhead > max_theoretical_quilt_size {
                anyhow::bail!(
                    "File '{}' with size {} exceeds Walrus theoretical maximum of {} for single file storage. \
                    This file cannot be stored in Walrus with the current shard configuration.",
                    res_data.full_path.display(),
                    ByteSize(file_size_with_overhead as u64),
                    ByteSize(max_theoretical_quilt_size as u64)
                );
            }

            // If file exceeds effective_quilt_size but is below theoretical limit,
            // place it alone in its own chunk and continue with the current chunk
            if file_size_with_overhead > effective_quilt_size {
                // Place large file in its own chunk (don't save current_chunk yet)
                chunks.push(vec![(res_data, quilt_input)]);
                // Continue filling the current chunk with remaining capacity
                continue;
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
    let path_str = rel_path
        .to_str()
        .ok_or(anyhow!("could not process the path string: {rel_path:?}"))?;

    // Normalize Windows path separators to URL-style forward slashes.
    // Only needed on Windows; on Unix, backslash can be a valid filename character.
    #[cfg(windows)]
    let path_str = path_str.replace('\\', "/");
    Ok(format!("/{}", path_str))
}

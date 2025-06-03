// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Functionality to read and check the files in of a website.

use std::{
    collections::BTreeSet,
    fmt::{self, Display},
    fs,
    io::Write,
    num::{NonZeroU16, NonZeroUsize},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};
use fastcrypto::hash::{HashFunction, Sha256};
use flate2::{write::GzEncoder, Compression};
use futures::stream::{self, StreamExt};
use move_core_types::u256::U256;
use regex::Regex;

use super::SiteData;
use crate::{
    publish::BlobManagementOptions,
    site::{config::WSResources, content::ContentType},
    types::{HttpHeaders, SuiResource, VecMap},
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

    /// Read a resource at a path.
    ///
    /// Ignores empty files.
    pub async fn read_resource(
        &self,
        full_path: &Path,
        resource_path: String,
    ) -> Result<Option<Resource>> {
        if let Some(ws_path) = &self.ws_resources_path {
            if full_path == ws_path {
                tracing::debug!(?full_path, "ignoring the ws-resources config file");
                return Ok(None);
            }
        }

        // Skip if resource matches ignore patterns/
        if self.is_ignored(&resource_path) {
            tracing::debug!(?resource_path, "ignoring resource due to ignore pattern");
            return Ok(None);
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
                    .filter(|(path, _)| Self::is_pattern_match(path, resource_path))
                    .max_by_key(|(path, _)| path.split('/').count())
                    .map(|(_, header_map)| header_map.0.clone())
            })
            .unwrap_or_default()
    }

    /// Matches a pattern to a resource path.
    ///
    /// The pattern can contain a wildcard `*` which matches any sequence of characters.
    /// e.g. `/foo/*` will match `/foo/bar` and `/foo/bar/baz`.
    fn is_pattern_match(pattern: &str, resource_path: &str) -> bool {
        let path_regex = pattern.replace('*', ".*");
        Regex::new(&path_regex)
            .map(|re| re.is_match(resource_path))
            .unwrap_or(false)
    }

    /// Returns true if the resource_path matches any of the ignore patterns.
    fn is_ignored(&self, resource_path: &str) -> bool {
        if let Some(ws_resources) = &self.ws_resources {
            if let Some(ignore_patterns) = &ws_resources.ignore {
                // Find the longest matching pattern
                return ignore_patterns
                    .iter()
                    .any(|pattern| Self::is_pattern_match(pattern, resource_path));
            }
        }
        false
    }

    /// Recursively iterate a directory and load all [`Resources`][Resource] within.
    pub async fn read_dir(&mut self, root: &Path) -> Result<SiteData> {
        let resource_paths = Self::iter_dir(root, root)?;
        if resource_paths.is_empty() {
            return Ok(SiteData::empty());
        }

        let futures = resource_paths
            .iter()
            .map(|(full_path, _)| {
                full_path_to_resource_path(full_path, root)
                    .map(|resource_path| self.read_resource(full_path, resource_path))
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

        Ok(SiteData::new(
            resources,
            self.ws_resources
                .as_ref()
                .and_then(|config| config.routes.clone()),
            self.ws_resources
                .as_ref()
                .and_then(|config| config.metadata.clone()),
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{HttpHeaders, ResourceManager};
    use crate::site::config::WSResources;

    struct PatternMatchTestCase {
        pattern: &'static str,
        path: &'static str,
        expected: bool,
    }

    #[test]
    fn test_is_pattern_match() {
        let tests = vec![
            PatternMatchTestCase {
                pattern: "/*.txt",
                path: "/file.txt",
                expected: true,
            },
            PatternMatchTestCase {
                pattern: "*.txt",
                path: "/file.doc",
                expected: false,
            },
            PatternMatchTestCase {
                pattern: "/test/*",
                path: "/test/file",
                expected: true,
            },
            PatternMatchTestCase {
                pattern: "/test/*",
                path: "/test/file.extension",
                expected: true,
            },
            PatternMatchTestCase {
                pattern: "/test/*",
                path: "/test/foo.bar.extension",
                expected: true,
            },
            PatternMatchTestCase {
                pattern: "/test/*",
                path: "/test/foo-bar_baz.extension",
                expected: true,
            },
            PatternMatchTestCase {
                pattern: "[invalid",
                path: "/file",
                expected: false,
            },
        ];
        for t in tests {
            assert_eq!(
                ResourceManager::is_pattern_match(t.pattern, t.path),
                t.expected
            );
        }
    }

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

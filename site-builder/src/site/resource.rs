use std::{
    collections::BTreeSet,
    fmt::{self, Display},
    fs::read_dir,
    io::Write,
    path::Path,
};

use anyhow::{anyhow, Result};
use flate2::{write::GzEncoder, Compression};
use move_core_types::u256::U256;
use sui_sdk::rpc_types::{SuiMoveStruct, SuiMoveValue};
use walrus_core::{
    encoding::{EncodingConfig, SliverPair},
    metadata::VerifiedBlobMetadataWithId,
    BlobId,
};

use crate::site::content::{ContentEncoding, ContentType};

#[derive(PartialEq, Eq, Debug, Clone, Ord, PartialOrd)]
pub struct ResourceInfo {
    pub path: String,
    pub content_type: ContentType,
    pub content_encoding: ContentEncoding,
    pub blob_id: BlobId,
}

impl TryFrom<&SuiMoveStruct> for ResourceInfo {
    type Error = anyhow::Error;

    fn try_from(source: &SuiMoveStruct) -> Result<Self, Self::Error> {
        let path = get_dynamic_field!(source, "path", SuiMoveValue::String)?;
        let content_type: ContentType =
            get_dynamic_field!(source, "content_type", SuiMoveValue::String)?.try_into()?;
        let content_encoding: ContentEncoding =
            get_dynamic_field!(source, "content_encoding", SuiMoveValue::String)?.try_into()?;
        let blob_id = blob_id_from_u256(
            get_dynamic_field!(source, "blob_id", SuiMoveValue::String)?.parse::<U256>()?,
        );
        Ok(Self {
            path,
            content_type,
            content_encoding,
            blob_id,
        })
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

/// A resource inside a site.
///
/// [`Resource`] objects are always compared on their `info` field
/// ([`ResourceInfo`]), and never on their `slivers` or `metadata`.
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Resource {
    pub info: ResourceInfo,
    pub slivers: Option<Vec<SliverPair>>,
    pub metadata: Option<VerifiedBlobMetadataWithId>,
    pub unencoded_size: usize,
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
            slivers: None,
            metadata: None,
            unencoded_size: 0,
        }
    }
}

impl Resource {
    pub fn new(
        path: String,
        content_type: ContentType,
        content_encoding: ContentEncoding,
        blob_id: BlobId,
        slivers: Option<Vec<SliverPair>>,
        metadata: Option<VerifiedBlobMetadataWithId>,
        unencoded_size: usize,
    ) -> Self {
        Resource {
            info: ResourceInfo {
                path,
                content_type,
                content_encoding,
                blob_id,
            },
            slivers,
            metadata,
            unencoded_size,
        }
    }
}

impl Display for Resource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Resource: {}, blob id: {})",
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
            ResourceOp::Deleted(r) => r,
            ResourceOp::Created(r) => r,
        }
    }
}

/// A summary of the operations performed by the site builder.
#[derive(Debug, Clone)]
pub struct OperationSummary {
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

pub struct OperationsSummary(pub Vec<OperationSummary>);

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
pub struct ResourceSet {
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

#[derive(Debug)]
pub struct ResourceManager {
    /// The resources in the site.
    pub resources: ResourceSet,
}

impl ResourceManager {
    pub fn new() -> Result<Self> {
        Ok(ResourceManager {
            resources: ResourceSet::default(),
        })
    }

    // /// List all names of resources stored in the [`ResourceManager`].
    // pub fn all_names(&self) -> Vec<String> {
    //     self.resources
    //         .inner
    //         .iter()
    //         .map(|res| res.info.path.clone())
    //         .collect()
    // }

    /// Read a resource at a path.
    ///
    /// Ignores empty files.
    pub fn read_resource(
        &self,
        full_path: &Path,
        root: &Path,
        content_encoding: &ContentEncoding,
        encoding_config: &EncodingConfig,
    ) -> Result<Option<Resource>> {
        let rel_path = full_path.strip_prefix(root)?;
        let content_type = ContentType::from_extension(
            full_path
                .extension()
                .ok_or(anyhow!("No extension found for {:?}", rel_path))?
                .to_str()
                .ok_or(anyhow!("Invalid extension"))?,
        );
        let plain_content = std::fs::read(full_path)?;
        if plain_content.is_empty() {
            // We are ignoring empty files.
            return Ok(None);
        }
        let content = match content_encoding {
            ContentEncoding::PlainText => plain_content,
            ContentEncoding::Gzip => compress(&plain_content)?,
        };

        let pathname = format!(
            "/{}",
            rel_path
                .to_str()
                .ok_or(anyhow!("could not process the path string: {:?}", rel_path))?
        );

        let (slivers, metadata) = encoding_config
            .get_blob_encoder(&content)?
            .encode_with_metadata();

        Ok(Some(Resource::new(
            pathname,
            content_type,
            *content_encoding,
            *metadata.blob_id(),
            Some(slivers),
            Some(metadata),
            content.len(),
        )))
    }

    /// Recursively iterate a directory and load all [`Resources`][Resource] within.
    pub fn read_dir(
        &mut self,
        root: &Path,
        content_encoding: &ContentEncoding,
        encoding_config: &EncodingConfig,
    ) -> Result<()> {
        self.resources =
            ResourceSet::from_iter(self.iter_dir(root, root, content_encoding, encoding_config)?);
        Ok(())
    }

    fn iter_dir(
        &self,
        start: &Path,
        root: &Path,
        content_encoding: &ContentEncoding,
        encoding_config: &EncodingConfig,
    ) -> Result<Vec<Resource>> {
        let mut resources: Vec<Resource> = vec![];
        let entries = read_dir(start)?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                resources.extend(self.iter_dir(&path, root, content_encoding, encoding_config)?);
            } else if let Some(res) =
                self.read_resource(&path, root, content_encoding, encoding_config)?
            {
                resources.push(res);
            }
        }
        Ok(resources)
    }
}

fn compress(content: &[u8]) -> Result<Vec<u8>> {
    if content.is_empty() {
        // Compression of an empty vector may result in compression headers
        return Ok(vec![]);
    }
    let mut encoder = GzEncoder::new(vec![], Compression::default());
    encoder.write_all(content)?;
    Ok(encoder.finish()?)
}

use std::{
    fmt::{self, Display},
    fs::read_dir,
    io::Write,
    path::Path,
};

use anyhow::{anyhow, Result};
use flate2::{write::GzEncoder, Compression};
use sui_sdk::rpc_types::{SuiMoveStruct, SuiMoveValue};

use super::{builder::BlocksiteCall, manager::OBJ_MARGIN};
use crate::site::{
    content::{ContentEncoding, ContentType},
    macros::get_dynamic_field,
    manager::{ARG_MARGIN, MAX_ARG_SIZE, MAX_OBJ_SIZE, MAX_TX_SIZE, TX_MARGIN},
};

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Resource {
    pub name: String,
    pub content_type: ContentType,
    pub content_encoding: ContentEncoding,
    pub parts: usize,
    pub content: Vec<u8>,
}

impl Resource {
    pub fn new(
        name: String,
        content_type: ContentType,
        content_encoding: ContentEncoding,
        parts: usize,
        content: Vec<u8>,
    ) -> Self {
        Resource {
            name,
            content_type,
            content_encoding,
            parts,
            content,
        }
    }

    pub fn read(
        full_path: &Path,
        root: &Path,
        content_encoding: &ContentEncoding,
    ) -> Result<Option<Vec<Self>>> {
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
            // We are ignoring empty files
            return Ok(None);
        }
        let content = match content_encoding {
            ContentEncoding::PlainText => plain_content,
            ContentEncoding::Gzip => compress(&plain_content)?,
        };
        Ok(Some(Resource::split_multi_resource(
            &format!(
                "/{}", // We want the path to be in the form `/path/to/resource.ext`
                rel_path.to_str().ok_or(anyhow!("Invalid path"))?
            ),
            content_type,
            content_encoding,
            &content,
        )))
    }

    /// Split a resource over [MAX_OBJ_SIZE] into multiple smaller resources
    fn split_multi_resource(
        rel_path: &str,
        content_type: ContentType,
        content_encoding: &ContentEncoding,
        content: &[u8],
    ) -> Vec<Resource> {
        let n_parts = content.len() / (MAX_OBJ_SIZE - OBJ_MARGIN) + 1;
        content
            .chunks(MAX_OBJ_SIZE - OBJ_MARGIN)
            .enumerate()
            .map(|(idx, chunk)| {
                Resource::new(
                    Resource::path_to_multi_resource_name(rel_path, idx),
                    content_type.clone(),
                    *content_encoding,
                    n_parts,
                    chunk.to_vec(),
                )
            })
            // .map()
            .collect::<Vec<_>>()
    }

    fn path_to_multi_resource_name(path: &str, number: usize) -> String {
        if number == 0 {
            return path.to_owned();
        }
        format!("part-{}{}", number, path)
    }

    /// Compute the (approximate) size of the resource when added to a PTB
    pub fn size_in_ptb(&self) -> usize {
        // TODO: check this approximation
        (1 + self.content.len() / (MAX_ARG_SIZE - ARG_MARGIN)) * MAX_ARG_SIZE
    }

    /// Get the "temporary path" for this resource
    /// The temporary path does not start with `/`, so that it will
    /// not be routable by the portal (the portal always prepends `/`
    /// to path names).
    pub fn tmp_path(&self) -> String {
        format!("tmp{}", self.name)
    }

    /// Get the series of move calls needed to push the resource's content
    /// The calls are already grouped by PTB.  If there is no content,
    /// no resource is created. TODO: This is a UX decision -- would
    /// anyone want to have "empty files" in their filetree?
    pub fn to_ptb_calls(&self) -> Result<Vec<Vec<BlocksiteCall>>> {
        if self.content.is_empty() {
            return Ok(vec![]);
        }
        let create = BlocksiteCall::new_resource_and_add(
            &self.tmp_path(),
            &self.content_type.to_string(),
            &self.content_encoding.to_string(),
            self.parts,
            &[],
        )?;

        let mut calls = self
            .content
            .chunks(MAX_TX_SIZE - TX_MARGIN)
            .map(|c| {
                c.chunks(MAX_ARG_SIZE - ARG_MARGIN)
                    .map(|piece| BlocksiteCall::add_piece_to_existing(&self.tmp_path(), piece))
                    .collect()
            })
            .collect::<Result<Vec<Vec<_>>>>()?;

        // Add the creation command in front
        calls
            .first_mut()
            .expect("There must be at least one vec of calls")
            .insert(0, create);

        // As a last step, move the resource at the right place
        calls
            .last_mut()
            .expect("There must be at least one vec of calls")
            .push(BlocksiteCall::move_resource(&self.tmp_path(), &self.name)?);

        Ok(calls)
    }

    /// Recursively iterate a directory and load all resources within
    pub fn iter_dir(root: &Path, content_encoding: &ContentEncoding) -> Result<Vec<Resource>> {
        Resource::inner_iter_dir(root, root, content_encoding)
    }

    fn inner_iter_dir(
        start: &Path,
        root: &Path,
        content_encoding: &ContentEncoding,
    ) -> Result<Vec<Resource>> {
        let mut resources: Vec<Resource> = vec![];
        let entries = read_dir(start).expect("Reading path failed. Please provide a valid path");
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                resources.extend(Resource::inner_iter_dir(&path, root, content_encoding)?);
            } else if let Some(res) = Resource::read(&path, root, content_encoding)? {
                resources.extend(res);
            }
        }
        Ok(resources)
    }
}

impl Display for Resource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Resource: {} ({} B)", self.name, self.content.len())
    }
}

impl TryFrom<SuiMoveStruct> for Resource {
    type Error = anyhow::Error;

    fn try_from(sui_move_struct: SuiMoveStruct) -> Result<Self, Self::Error> {
        let name = get_dynamic_field!(sui_move_struct, "name", SuiMoveValue::String);
        let content_type =
            get_dynamic_field!(sui_move_struct, "content_type", SuiMoveValue::String);
        let content_encoding =
            get_dynamic_field!(sui_move_struct, "content_encoding", SuiMoveValue::String);
        let parts =
            get_dynamic_field!(sui_move_struct, "parts", SuiMoveValue::String).parse::<usize>()?;
        let content = get_dynamic_field!(sui_move_struct, "contents", SuiMoveValue::Vector)
            .iter()
            .map(|v| match v {
                SuiMoveValue::Number(x) => Ok(*x as u8), // TODO: there must be a better way
                _ => Err(anyhow!("Could not convert to vec<u8>")),
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Resource::new(
            name,
            content_type.try_into()?,
            content_encoding.try_into()?,
            parts,
            content,
        ))
    }
}

#[derive(Default, Clone)]
pub struct ResourceManager {
    /// The resources that fit into a single PTB
    pub single_ptb: Vec<Resource>,
    /// The resources that span multiple PTBs
    pub multi_ptb: Vec<Resource>,
}

impl ResourceManager {
    /// Schedule the resource for publishing depending on its size
    pub fn add_resource(&mut self, resource: Resource) {
        if resource.size_in_ptb() < MAX_TX_SIZE {
            self.single_ptb.push(resource);
        } else {
            self.multi_ptb.push(resource);
        }
    }

    /// Get an iterator over vectors of resources that fit into a single PTB
    pub fn group_by_ptb(&mut self) -> impl Iterator<Item = Vec<Resource>> {
        let mut iter: Vec<Vec<_>> = vec![];
        let mut current_ptb_size = 0;
        self.single_ptb
            .sort_unstable_by_key(|first| first.size_in_ptb());
        for resource in self.single_ptb.drain(..) {
            let required_space = resource.size_in_ptb();
            if iter.is_empty() || current_ptb_size + required_space > MAX_TX_SIZE {
                iter.push(vec![resource]);
                current_ptb_size = required_space;
            } else {
                iter.last_mut().unwrap().push(resource);
                current_ptb_size += required_space;
            }
        }
        iter.into_iter()
    }

    /// List all names of resources stored in the [ResourceManager]
    pub fn all_names(&self) -> Vec<String> {
        self.single_ptb
            .iter()
            .chain(self.multi_ptb.iter())
            .map(|res| res.name.clone())
            .collect()
    }

    pub fn total_size(&self) -> usize {
        self.multi_ptb
            .iter()
            .chain(self.single_ptb.iter())
            .fold(0, |size, r| size + r.content.len())
    }

    pub fn get_resource_by_name(&self, name: &str) -> Option<&Resource> {
        self.single_ptb
            .iter()
            .chain(self.multi_ptb.iter())
            .find(|res| res.name == name)
    }
}

impl Display for ResourceManager {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.single_ptb.is_empty() && self.multi_ptb.is_empty() {
            write!(f, "No Resources")
        } else {
            write!(
                f,
                "Resources for a total of {} B:\n  - {}",
                self.total_size(),
                self.multi_ptb
                    .iter()
                    .chain(self.single_ptb.iter())
                    .map(|r| r.to_string())
                    .collect::<Vec<_>>()
                    .join("\n  - ")
            )
        }
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

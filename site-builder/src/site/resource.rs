use std::{
    fmt::{self, Display},
    fs::read_dir,
    io::Write,
    path::Path,
};

use anyhow::{anyhow, ensure, Result};
use flate2::{write::GzEncoder, Compression};

use crate::site::{
    content::{ContentEncoding, ContentType},
    manager::{ARG_MARGIN, MAX_ARG_SIZE, MAX_OBJ_SIZE, MAX_TX_SIZE},
};

#[derive(PartialEq, Eq, Debug)]
pub struct Resource {
    pub name: String,
    pub content_type: ContentType,
    pub content_encoding: ContentEncoding,
    pub content: Vec<u8>,
}

impl Resource {
    pub fn new(
        name: String,
        content_type: ContentType,
        content_encoding: ContentEncoding,
        content: Vec<u8>,
    ) -> Self {
        Resource {
            name,
            content_type,
            content_encoding,
            content,
        }
    }

    pub fn read(full_path: &Path, root: &Path, content_encoding: &ContentEncoding) -> Result<Self> {
        let rel_path = full_path.strip_prefix(root)?;
        let content_type = ContentType::from_extension(
            full_path
                .extension()
                .ok_or(anyhow!("No extension found for {:?}", rel_path))?
                .to_str()
                .ok_or(anyhow!("Invalid extension"))?,
        );
        let plain_content = std::fs::read(full_path)?;
        let content = match content_encoding {
            ContentEncoding::PlainText => plain_content,
            ContentEncoding::Gzip => compress(&plain_content)?,
        };
        ensure!(
            content.len() < MAX_OBJ_SIZE,
            "Resource {:?} is too large, with size {}",
            rel_path,
            content.len(),
        );
        Ok(Resource::new(
            format!(
                "/{}", // We want the path to be in the form `/path/to/resource.ext`
                rel_path.to_str().ok_or(anyhow!("Invalid path"))?
            ),
            content_type,
            *content_encoding,
            content,
        ))
    }

    /// Compute the (approximate) size of the resource when added to a PTB
    pub fn size_in_ptb(&self) -> usize {
        // TODO: check this approximation
        (1 + self.content.len() / (MAX_ARG_SIZE - ARG_MARGIN)) * MAX_ARG_SIZE
    }

    /// Get the "temporary path" for this resource
    pub fn tmp_path(&self) -> String {
        format!("/tmp{}", self.name)
    }

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
            } else {
                resources.push(Resource::read(&path, root, content_encoding)?);
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

#[derive(Default)]
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

    pub fn total_size(&self) -> usize {
        self.multi_ptb
            .iter()
            .chain(self.single_ptb.iter())
            .fold(0, |size, r| size + r.content.len())
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
    let mut encoder = GzEncoder::new(vec![], Compression::default());
    encoder.write_all(content)?;
    Ok(encoder.finish()?)
}

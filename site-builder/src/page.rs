use std::{io::Write, path::PathBuf};

use anyhow::{anyhow, Result};

use crate::content::{ContentEncoding, ContentType};

pub struct Site {
    pub name: String,
}

impl Site {
    pub fn new(name: &str) -> Self {
        Site {
            name: name.to_owned(),
        }
    }
}

#[derive(Debug)]
pub struct Page {
    pub name: String,
    pub content_type: ContentType,
    pub content_encoding: ContentEncoding,
    pub content: Vec<u8>,
}

impl Page {
    fn new(
        name: String,
        content_type: ContentType,
        content_encoding: ContentEncoding,
        content: Vec<u8>,
    ) -> Self {
        Page {
            name,
            content_type,
            content_encoding,
            content,
        }
    }

    /// Create a page from a file
    /// The root is needed to calculate the relative path in the site hierarchy
    pub fn read(
        full_path: &PathBuf,
        root: &PathBuf,
        content_encoding: &ContentEncoding,
    ) -> Result<Self> {
        let rel_path = full_path.strip_prefix(root)?;
        let content_type = ContentType::from_extension(
            full_path
                .extension()
                .ok_or(anyhow!("No extension found"))?
                .to_str()
                .ok_or(anyhow!("Invalid extension"))?,
        );
        let plain_content = std::fs::read(full_path)?;
        let content = match content_encoding {
            ContentEncoding::PlainText => plain_content,
            ContentEncoding::Gzip => compress(&plain_content)?,
        };
        Ok(Page::new(
            rel_path
                .to_str()
                .ok_or(anyhow!("Invalid path"))?
                .to_string(),
            content_type,
            content_encoding.clone(),
            content,
        ))
    }
}

use flate2::{write::GzEncoder, Compression};

fn compress(content: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = GzEncoder::new(vec![], Compression::default());
    encoder.write_all(content)?;
    Ok(encoder.finish()?)
}

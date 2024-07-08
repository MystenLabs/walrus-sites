// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Pre-process a directory tree adding index files to make them browsable.
//!
//! The look and feel is taken from Python's `list_directory` method in `http.server`.

use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::site::resource::full_path_to_resource_path;

pub struct Preprocessor;

impl Preprocessor {
    pub fn iter_dir(path: &Path) -> Result<Vec<DirNode>> {
        let mut nodes = vec![];
        debug_assert!(path.is_dir());
        let items = std::fs::read_dir(path)?;
        let mut cur_node = DirNode::new(path.to_path_buf());
        for item in items.flatten() {
            let item_path = item.path();
            // Ignore index files.
            if item_path
                .file_name()
                .is_some_and(|name| name == "index.html")
            {
                continue;
            }
            cur_node.contents.push(item_path.to_path_buf());
            if item_path.is_dir() {
                let sub_nodes = Self::iter_dir(&item_path)?;
                nodes.extend(sub_nodes);
            }
        }
        nodes.push(cur_node);
        Ok(nodes)
    }

    pub fn preprocess(path: &Path) -> Result<()> {
        for node in Self::iter_dir(path)? {
            node.write_index(path)?;
        }
        Ok(())
    }
}

/// A directory in the directory tree of the preprocessor.
#[derive(Debug)]
pub struct DirNode {
    /// The paths of files in the directory.
    contents: Vec<PathBuf>,
    path: PathBuf,
}

impl DirNode {
    pub fn new(path: PathBuf) -> Self {
        Self {
            contents: vec![],
            path,
        }
    }

    fn index_path(&self) -> PathBuf {
        self.path.join("index.html")
    }

    fn path_to_html(path: &Path, root: &Path) -> Result<String> {
        let mut link_name = path
            .file_name()
            .ok_or(anyhow::anyhow!(
                "no valid filename found for path {}; root `/` and `..` paths are not allowed",
                path.to_string_lossy(),
            ))?
            .to_string_lossy()
            .to_string();

        let actual_link = if path.is_dir() {
            link_name.push('/');
            path.join("index.html")
        } else {
            path.to_path_buf()
        };

        let resource_path = full_path_to_resource_path(&actual_link, root)?;
        Ok(format!("<a href=\"{}\">{}</a>", resource_path, link_name,))
    }

    fn to_html(&self, root: &Path) -> Result<String> {
        let title_string = format!(
            "Directory listing for {}",
            full_path_to_resource_path(&self.path, root)?
        );

        let mut contents: Vec<String> = self
            .contents
            .iter()
            .map(|p| Ok(format!("<li>{}</li>", Self::path_to_html(p, root)?)))
            .collect::<Result<_>>()?;
        contents.sort();
        let body = contents.join("\n");

        Ok(format!(
            "<!DOCTYPE html>\n<html>\n<head>\n<title>{title_string}</title>\n</head>\n\
            <body>\n<h1>{title_string}</h1>\n<hr>\n<ul>\n{body}\n</ul>\n<hr>\n</body>\n</html>",
        ))
    }

    pub fn write_index(&self, root: &Path) -> Result<()> {
        let html = self.to_html(root)?;
        std::fs::write(self.index_path(), html)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dirnode_to_html() {
        let dir = DirNode {
            contents: vec![
                PathBuf::from("/my/sub/path"),
                PathBuf::from("/my/sub/another"),
            ],
            path: PathBuf::from("/my/sub"),
        };
        let expected = r#"<!DOCTYPE html>
<html>
<head>
<title>Directory listing for /sub</title>
</head>
<body>
<h1>Directory listing for /sub</h1>
<hr>
<ul>
<li><a href="/sub/another">another</a></li>
<li><a href="/sub/path">path</a></li>
</ul>
<hr>
</body>
</html>"#;
        assert_eq!(dir.to_html(Path::new("/my/")).unwrap(), expected);
    }
}

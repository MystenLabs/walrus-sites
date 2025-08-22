// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::BTreeMap,
    fs,
    num::NonZeroUsize,
    path::{Path, PathBuf},
};

use anyhow::anyhow;
use fastcrypto::hash::{HashFunction, Sha256};
use move_core_types::u256::U256;
use regex::Regex;

use super::{full_path_to_resource_path, LocalResource};
use crate::{
    site::content::ContentType,
    types::{HttpHeaders, VecMap},
};

/// Loads and manages the set of resources composing the site.
#[derive(Debug)]
pub(crate) struct ResourceManager {
    /// The ws-resources.json contents.
    // pub ws_resources: Option<WSResources>,
    pub ws_res_headers: BTreeMap<String, HttpHeaders>,
    pub ignore_patterns: Vec<String>,
    /// The ws-resource file path.
    pub ws_resources_path: Option<PathBuf>,
    /// The maximum number of concurrent calls to the walrus cli for computing the blob ID.
    pub max_concurrent: Option<NonZeroUsize>,
}

impl ResourceManager {
    pub fn new(
        mut ws_res_headers: BTreeMap<String, HttpHeaders>,
        ignore_patterns: Vec<String>,
        // ws_resources: Option<WSResources>,
        ws_resources_path: Option<PathBuf>,
        max_concurrent: Option<NonZeroUsize>,
    ) -> Self {
        // TODO(nikos): Dedup ignore_patterns
        // TODO(nikos): Test if this stil works
        // Cast the keys to lowercase because http headers
        //  are case-insensitive: RFC7230 sec. 2.7.3
        for (_, header_map) in ws_res_headers.iter_mut() {
            header_map.0 = header_map
                .0
                .iter()
                .map(|(k, v)| (k.to_lowercase(), v.clone()))
                .collect();
        }

        ResourceManager {
            ws_res_headers,
            ignore_patterns,
            ws_resources_path,
            max_concurrent,
        }
    }

    /// Recursively iterate a directory and load all [`Resources`][Resource] within.
    pub fn read_dir(&mut self, root: &Path) -> anyhow::Result<Vec<LocalResource>> {
        let resource_paths = Self::iter_dir(root, root)?;

        if resource_paths.is_empty() {
            return Ok(vec![]);
        }

        let resources: Vec<LocalResource> = resource_paths
            .iter()
            .map(|(full_path, _)| {
                let resource_path = full_path_to_resource_path(full_path, root)?;
                self.read_resource(full_path, resource_path)
            })
            .try_fold(
                Vec::new(),
                |mut acc, res_opt| -> anyhow::Result<Vec<LocalResource>> {
                    if let Some(item) = res_opt? {
                        acc.push(item);
                    }
                    Ok(acc)
                },
            )?;

        Ok(resources)
    }

    /// Read a resource at a path.
    ///
    /// Ignores empty files.
    pub fn read_resource(
        &self,
        full_path: &Path,
        resource_path: String,
    ) -> anyhow::Result<Option<LocalResource>> {
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
            ResourceManager::derive_http_headers(&self.ws_res_headers, &resource_path);
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

        Ok(Some(LocalResource::new(
            resource_path,
            full_path.to_owned(),
            HttpHeaders(http_headers),
            U256::from_le_bytes(&blob_hash),
            plain_content.len(),
        )))
    }

    ///  Derives the HTTP headers for a resource based on the ws-resources.yaml.
    ///
    ///  Matches the path of the resource to the wildcard paths in the configuration to
    ///  determine the headers to be added to the HTTP response.
    pub fn derive_http_headers(
        ws_res_headers: &BTreeMap<String, HttpHeaders>,
        resource_path: &str,
    ) -> VecMap<String, String> {
        ws_res_headers
            .iter()
            .filter(|(path, _)| Self::is_pattern_match(path, resource_path))
            .max_by_key(|(path, _)| path.split('/').count())
            .map(|(_, header_map)| header_map.0.clone())
            .unwrap_or_default()
    }

    /// Matches a pattern to a resource path.
    ///
    /// The pattern can contain a wildcard `*` which matches any sequence of characters.
    /// e.g. `/foo/*` will match `/foo/bar` and `/foo/bar/baz`.
    fn is_pattern_match(pattern: &str, resource_path: &str) -> bool {
        // TODO(nikos): Use glob instead of regex
        let path_regex = pattern.replace('*', ".*");
        Regex::new(&path_regex)
            .map(|re| re.is_match(resource_path))
            .unwrap_or(false)
    }

    /// Returns true if the resource_path matches any of the ignore patterns.
    fn is_ignored(&self, resource_path: &str) -> bool {
        return self
            .ignore_patterns
            .iter()
            .any(|pattern| Self::is_pattern_match(pattern, resource_path));
    }

    fn iter_dir(start: &Path, root: &Path) -> anyhow::Result<Vec<(PathBuf, PathBuf)>> {
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

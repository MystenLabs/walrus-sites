// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{collections::BTreeMap, path::Path};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::Routes;
use crate::types::{HttpHeaders, Metadata};

/// Deserialized object of the file's `ws-resource.json` contents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WSResources {
    /// The HTTP headers to be set for the resources.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<BTreeMap<String, HttpHeaders>>,
    /// The HTTP headers to be set for the resources.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routes: Option<Routes>,
    /// The attributes used inside the Display object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,
    /// The name of the site.
    #[serde(rename = "site-name", skip_serializing_if = "Option::is_none")]
    pub site_name: Option<String>,
}

impl WSResources {
    /// Reads and parses the `ws-resources.json` file into a `WSResources` struct.
    pub fn read<P: AsRef<Path>>(path: P) -> Result<WSResources> {
        // Load the JSON contents to a string.
        tracing::info!(file=%path.as_ref().display(), "reading Walrus site resources");
        let file_contents =
            std::fs::read_to_string(path).context("Failed to read ws_config.json")?;
        // Read the JSON contents of the file as an instance of `WSResources`.
        let ws_config: WSResources = serde_json::from_str(&file_contents)?;
        tracing::info!(?ws_config, "ws resources configuration loaded");
        Ok(ws_config)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    const HEADER_DATA: &str = r#"
            "headers": {
                "/index.html": {
                    "Content-Type": "application/json",
                    "Content-Encoding": "gzip",
                    "Cache-Control": "no-cache"
                }
            }
        "#;

    const ROUTE_DATA: &str = r#"
            "routes": {
                "/*": "/index.html"
            }
        "#;

    const METADATA: &str = r#"
        "metadata": {
            "link": "https://subdomain.wal.app",
            "image_url": "https://subdomain.wal.app/image.png",
            "description": "This is walrus site.",
            "project_url": "https://github.com/MystenLabs/walrus-sites/",
            "creator": "MystenLabs"
        }
    "#;

    #[test]
    fn test_read_ws_resources() {
        let header_data = format!("{{{}}}", HEADER_DATA);
        serde_json::from_str::<WSResources>(&header_data).expect("parsing should succeed");
        let route_data = format!("{{{}}}", ROUTE_DATA);
        serde_json::from_str::<WSResources>(&route_data).expect("parsing should succeed");
        let route_header_data = format!("{{{},{}}}", HEADER_DATA, ROUTE_DATA);
        serde_json::from_str::<WSResources>(&route_header_data).expect("parsing should succeed");
        let all_fields_included = format!("{{{},{},{}}}", HEADER_DATA, ROUTE_DATA, METADATA);
        serde_json::from_str::<WSResources>(&all_fields_included).expect("parsing should succeed");
    }
}

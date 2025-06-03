// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{collections::BTreeMap, path::Path};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sui_types::base_types::ObjectID;

use super::Routes;
use crate::types::{HttpHeaders, Metadata};

/// Deserialized object of the file's `ws-resource.json` contents.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site_name: Option<String>,
    /// The object ID of the published site.
    ///
    /// This parameter is automatically set by the `deploy` command to store
    /// the information about the Site object being used, so there is no need
    /// to manually keep track of it.
    /// On subsequent calls to the `deploy` command, this parameter is used
    /// to update the site.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_id: Option<ObjectID>,
    /// The paths to ignore when publishing/updating.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore: Option<Vec<String>>,
}

impl WSResources {
    /// Reads and parses the `ws-resources.json` file into a `WSResources` struct.
    pub fn read<P: AsRef<Path>>(path: P) -> Result<WSResources> {
        // Load the JSON contents to a string.
        tracing::info!(file=%path.as_ref().display(), "reading Walrus site resources");
        let file_contents =
            std::fs::read_to_string(path).context("Failed to read ws_config.json")?;
        // Read the JSON contents of the file as an instance of `WSResources`.
        let ws_config: WSResources = serde_json::from_str(&file_contents)
            .context(format!(
                "Failed to parse ws_resources.json: {file_contents}\n\nCheck for typos or unknown fields (e.g. 'site-name' with a dash for the site name field; use 'site_name' instead)."
            ))?;

        tracing::info!(?ws_config, "ws resources configuration loaded");
        Ok(ws_config)
    }

    /// Saves the `WSResources` struct to a json file, pretty-printed.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        tracing::info!(file=%path.as_ref().display(), "saving Walrus site resources");
        let file = std::fs::File::create(path.as_ref()).context(format!(
            "Failed to create file: {}",
            path.as_ref().display()
        ))?;

        let writer = std::io::BufWriter::new(file);
        serde_json::to_writer_pretty(writer, self).context(format!(
            "Failed to write to file: {}",
            path.as_ref().display()
        ))?;
        Ok(())
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

    const SITE_NAME_INVALID_FIELD: &str = r#"
    "site-name": "A Walrus Site Name"
    "#;

    const SITE_NAME_DATA: &str = r#"
    "site_name": "A Walrus Site Name"
    "#;

    const IGNORE_DATA: &str = r#"
    "ignore": [
        "/foo/*",
        "/baz/bar/*"
    ]
    "#;

    #[test]
    fn test_read_ws_resources() {
        let header_data = format!("{{{HEADER_DATA}}}");
        serde_json::from_str::<WSResources>(&header_data).expect("parsing should succeed");
        let route_data = format!("{{{ROUTE_DATA}}}");
        serde_json::from_str::<WSResources>(&route_data).expect("parsing should succeed");
        let route_header_data = format!("{{{HEADER_DATA},{ROUTE_DATA}}}");
        serde_json::from_str::<WSResources>(&route_header_data).expect("parsing should succeed");
        let all_fields_included = format!("{{{HEADER_DATA},{ROUTE_DATA},{METADATA}}}");
        serde_json::from_str::<WSResources>(&all_fields_included).expect("parsing should succeed");
        // Test for ignore field
        let ignore_data = format!("{{{IGNORE_DATA}}}");
        let parsed: WSResources =
            serde_json::from_str(&ignore_data).expect("parsing should succeed");
        assert!(parsed.ignore.is_some());
        assert_eq!(parsed.ignore.unwrap(), vec!["/foo/*", "/baz/bar/*"]);
        // Test for invalid site name field
        let invalid_site_name_data = format!("{{{SITE_NAME_INVALID_FIELD}}}");
        // Parsing should fail
        let result = serde_json::from_str::<WSResources>(&invalid_site_name_data);
        assert!(result.is_err());
        // Test for valid site_name field
        let valid_site_name_data = format!("{{{SITE_NAME_DATA}}}");
        let parsed: WSResources =
            serde_json::from_str(&valid_site_name_data).expect("parsing should succeed");
        assert!(parsed.site_name.is_some());
        assert_eq!(parsed.site_name.unwrap(), "A Walrus Site Name".to_string());
    }
}

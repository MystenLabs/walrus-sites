// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, path::Path};

use anyhow::{Context, Result};
use serde::Deserialize;

use super::resource::HttpHeaders;

/// Deserialized object of the file's `ws-resource.json` contents.
#[derive(Deserialize, Debug)]
pub struct WSResources {
    pub headers: Option<HashMap<String, HttpHeaders>>,
    // TODO: "routes"" for client-side routing.
}

impl WSResources {
    /// Reads and parses the `ws-resources.json` file into a `WSResources` struct.
    pub fn read<P: AsRef<Path>>(path: P) -> Result<WSResources> {
        // Load the JSON contents to a string.
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

    #[test]
    fn test_read_ws_resources() {
        let data = r#"
        {
            "headers": {
                "/index.html": {
                    "Content-Type": "application/json",
                    "Content-Encoding": "gzip",
                    "Cache-Control": "no-cache"
                }
            }
        }
        "#;
        serde_json::from_str::<WSResources>(data).expect("parsing should succeed");
    }
}

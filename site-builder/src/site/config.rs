// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, path::Path};

use anyhow::{Context, Result};
use serde::Deserialize;

/// Deserialized object of the file's `ws-resource.json` contents.
#[derive(Deserialize, Debug)]
pub struct WSResources {
    pub headers: Option<HashMap<String, HashMap<String, String>>>,
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
        println!("ws-resources.json loaded! contents: {:?}", ws_config);
        Ok(ws_config)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::NamedTempFile;

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

        // Create a temporary file and write the test data to it.
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", data).unwrap();

        // Read the configuration from the temporary file.
        let result = WSResources::read(temp_file.path()).unwrap();
        println!("{:#?}", result);
    }
}

// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, error::Error, fs::File, io::BufReader, path::Path};

use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct WSConfig {
    headers: Option<HashMap<String, HashMap<String, String>>>,
    // TODO: "routes"" for client-side routing.
}

pub fn read_ws_config<P: AsRef<Path>>(path: P) -> Result<WSConfig, Box<dyn Error>> {
    // Open the file in read-only mode with buffer.
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    // Read the JSON contents of the file as an instance of `WSConfig`.
    let ws_config: WSConfig = serde_json::from_reader(reader)?;

    Ok(ws_config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_read_ws_config() {
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
        let result = read_ws_config(temp_file.path()).unwrap();
        println!("{:#?}", result);
    }
}

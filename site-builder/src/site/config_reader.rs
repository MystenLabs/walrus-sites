// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, error::Error, fs::File, io::BufReader, path::Path};

use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct WSConfig {
    headers: Option<HashMap<String, HashMap<String, String>>>,
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

    #[test]
    fn test_read_ws_config() {
        let path = "ws-config.json";
        let result = read_ws_config(path).unwrap();
        println!("{:#?}", result);
    }
}

// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{collections::BTreeMap, path::Path};

use anyhow::{Context, Result};
use move_core_types::u256::U256;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_with::{serde_as, DisplayFromStr};

use super::Routes;
use crate::{
    types::{HttpHeaders, Range, SuiResource},
    walrus::types::BlobId,
};

// HACK(giac): this is just to allow easy parsing of local resources.
/// Information about a resource.
///
/// This struct mirrors the information that is stored on chain.
#[serde_as]
#[derive(PartialEq, Eq, Debug, Clone, PartialOrd, Ord, Serialize, Deserialize)]
pub(crate) struct LocalResource {
    /// The relative path the resource will have on Sui.
    pub path: String,
    /// Response, Representation and Payload headers.
    pub headers: HttpHeaders,
    /// The blob ID of the resource.
    #[serde_as(as = "DisplayFromStr")]
    pub blob_id: BlobId,
    /// The hash of the blob contents. Serialze and deserialize as hex string.
    #[serde(
        serialize_with = "serialize_u256",
        deserialize_with = "deserialize_u256"
    )]
    pub blob_hash: U256,
    /// Byte ranges for the resource.
    pub range: Option<Range>,
}

impl From<LocalResource> for SuiResource {
    fn from(resource: LocalResource) -> Self {
        SuiResource {
            path: resource.path,
            headers: resource.headers,
            blob_id: resource.blob_id,
            blob_hash: resource.blob_hash,
            range: resource.range,
        }
    }
}

impl From<SuiResource> for LocalResource {
    fn from(resource: SuiResource) -> Self {
        LocalResource {
            path: resource.path,
            headers: resource.headers,
            blob_id: resource.blob_id,
            blob_hash: resource.blob_hash,
            range: resource.range,
        }
    }
}

fn deserialize_u256<'de, D>(deserializer: D) -> Result<U256, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    U256::from_str_radix(&s, 16).map_err(serde::de::Error::custom)
}

fn serialize_u256<S>(value: &U256, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&format!("{:x}", value))
}

/// Deserialized object of the file's `ws-resource.json` contents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WSResources {
    /// The HTTP headers to be set for the resources.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<BTreeMap<String, HttpHeaders>>,
    /// The HTTP headers to be set for the resources.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routes: Option<Routes>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_sui_from_local"
    )]
    pub pre_built: Option<Vec<SuiResource>>,
}

fn deserialize_sui_from_local<'de, D>(deserializer: D) -> Result<Option<Vec<SuiResource>>, D::Error>
where
    D: Deserializer<'de>,
{
    let resources: Option<Vec<LocalResource>> =
        Deserialize::deserialize(deserializer).unwrap_or(None);
    if let Some(res) = resources {
        Ok(Some(res.into_iter().map(SuiResource::from).collect()))
    } else {
        Ok(None)
    }
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

    use std::str::FromStr;

    use move_core_types::u256::U256;

    use super::*;
    use crate::walrus::types::BlobId;

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

    const PRE_BUILT_DATA: &str = r#"
        "pre_built": [
            {
                "path": "/index.html",
                "headers": {
                    "Cache-Control": "no-cache",
                    "Content-Encoding": "gzip",
                    "Content-Type": "application/json"
                },
                "blob_id": "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE",
                "blob_hash": "1869f",
                "range": null
            }
        ]
        "#;

    #[test]
    fn test_deserialize_resource() {
        let resource = LocalResource {
            path: "/index.html".to_owned(),
            blob_hash: U256::from_str("99999").unwrap(),
            blob_id: BlobId([1u8; 32]),
            range: None,
            headers: HttpHeaders(
                vec![
                    ("Content-Type".to_owned(), "application/json".to_owned()),
                    ("Content-Encoding".to_owned(), "gzip".to_owned()),
                    ("Cache-Control".to_owned(), "no-cache".to_owned()),
                ]
                .into_iter()
                .collect(),
            ),
        };

        let serialized = serde_json::to_string(&resource).expect("serialization should succeed");
        println!("{}", serialized);

        let _: LocalResource =
            serde_json::from_str(&serialized).expect("deserialization should succeed");
    }

    #[test]
    fn test_read_ws_resources() {
        let header_data = format!("{{{}}}", HEADER_DATA);
        serde_json::from_str::<WSResources>(&header_data).expect("parsing should succeed");
        let route_data = format!("{{{}}}", ROUTE_DATA);
        serde_json::from_str::<WSResources>(&route_data).expect("parsing should succeed");
        let pre_built_data = format!("{{{}}}", PRE_BUILT_DATA);
        serde_json::from_str::<WSResources>(&pre_built_data).expect("parsing should succeed");
    }
}

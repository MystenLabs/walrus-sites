//! The output of running commands on the Walrus CLI.
use std::{path::PathBuf, process::Output};

use anyhow::{anyhow, Result};
use serde::{de::DeserializeOwned, Deserialize};
use serde_with::{base64::Base64, serde_as, DisplayFromStr};
use sui_types::base_types::ObjectID;

use super::types::BlobId;

/// The output of the `store` command.
#[serde_as]
#[derive(Debug, Clone, Deserialize, Eq, PartialEq)]
pub struct StoreOutput {
    #[serde_as(as = "DisplayFromStr")]
    pub blob_id: BlobId,
    pub sui_object_id: ObjectID,
    pub blob_size: u64,
}

/// The output of the `read` command.
#[serde_as]
#[derive(Debug, Clone, Deserialize, Eq, PartialEq)]
pub struct ReadOutput {
    pub out: Option<PathBuf>,
    #[serde_as(as = "DisplayFromStr")]
    pub blob_id: BlobId,
    // When serializing to JSON, the blob is encoded as Base64 string.
    #[serde_as(as = "Base64")]
    pub blob: Vec<u8>,
}

/// The output of the `blob-id` command.
#[serde_as]
#[derive(Debug, Clone, Deserialize, Eq, PartialEq)]
pub struct BlobIdOutput {
    #[serde_as(as = "DisplayFromStr")]
    pub blob_id: BlobId,
    pub file: PathBuf,
    pub unencoded_length: u64,
}

pub fn try_from_output<T: DeserializeOwned>(output: Output) -> Result<T> {
    if !output.status.success() {
        // Format stderr as lossy utf8 to get the error print of the CLI.
        return Err(anyhow!(
            "running the command exited with error: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    };
    Ok(serde_json::from_str(&String::from_utf8(output.stdout)?)?)
}

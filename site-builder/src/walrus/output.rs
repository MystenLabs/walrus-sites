// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! The output of running commands on the Walrus CLI.

use std::{num::NonZeroU16, path::PathBuf, process::Output, time::Duration};

use anyhow::{anyhow, Context, Result};
use serde::{de::DeserializeOwned, Deserialize};
use serde_with::{base64::Base64, serde_as, DisplayFromStr};
use sui_types::{base_types::ObjectID, event::EventID};

use super::types::BlobId;

pub type Epoch = u64;
pub type EpochCount = u32;

/// Either an event ID or an object ID.
#[derive(Debug, Clone, Deserialize)]
#[allow(unused)]
pub enum EventOrObjectId {
    /// The variant representing an event ID.
    Event(EventID),
    /// The variant representing an object ID.
    Object(ObjectID),
}

/// The operation performed on blob and storage resources to register a blob.
#[derive(Debug, Clone, Deserialize)]
#[allow(unused)]
pub enum RegisterBlobOp {
    /// The storage and blob resources are purchased from scratch.
    RegisterFromScratch {
        encoded_length: u64,
        epochs_ahead: EpochCount,
    },
    /// The storage is reused, but the blob was not registered.
    ReuseStorage { encoded_length: u64 },
    /// A registration was already present.
    ReuseRegistration { encoded_length: u64 },
}

/// Result when attempting to store a blob.
#[serde_as]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
#[allow(unused)]
pub enum BlobStoreResult {
    /// The blob already exists within Walrus, was certified, and is stored for at least the
    /// intended duration.
    AlreadyCertified {
        /// The blob ID.
        #[serde_as(as = "DisplayFromStr")]
        blob_id: BlobId,
        /// The event where the blob was certified.
        event_or_object: EventOrObjectId,
        /// The epoch until which the blob is stored (exclusive).
        end_epoch: Epoch,
    },
    /// The blob was newly created; this contains the newly created Sui object associated with the
    /// blob.
    NewlyCreated {
        /// The Sui blob object that holds the newly created blob.
        blob_object: Blob,
        /// The encoded size, including metadata.
        resource_operation: RegisterBlobOp,
        /// The storage cost, excluding gas.
        cost: u64,
    },
    /// The blob is known to Walrus but was marked as invalid.
    ///
    /// This indicates a bug within the client, the storage nodes, or more than a third malicious
    /// storage nodes.
    MarkedInvalid {
        /// The blob ID.
        #[serde_as(as = "DisplayFromStr")]
        blob_id: BlobId,
        /// The event where the blob was marked as invalid.
        event: EventID,
    },
}

impl BlobStoreResult {
    /// Returns the blob ID.
    #[allow(dead_code)]
    pub fn blob_id(&self) -> &BlobId {
        match self {
            Self::AlreadyCertified { blob_id, .. } => blob_id,
            Self::MarkedInvalid { blob_id, .. } => blob_id,
            Self::NewlyCreated {
                blob_object: Blob { blob_id, .. },
                ..
            } => blob_id,
        }
    }
}

/// Supported Walrus encoding types.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Default, Deserialize)]
#[repr(u8)]
pub enum EncodingType {
    /// Default RaptorQ encoding.
    #[default]
    RedStuff = 0,
}

/// Sui object for storage resources.
#[derive(Debug, PartialEq, Eq, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageResource {
    /// Object ID of the Sui object.
    pub id: ObjectID,
    /// The start epoch of the resource (inclusive).
    pub start_epoch: Epoch,
    /// The end epoch of the resource (exclusive).
    pub end_epoch: Epoch,
    /// The total amount of reserved storage.
    pub storage_size: u64,
}

/// Sui object for a blob.
#[serde_as]
#[derive(Debug, PartialEq, Eq, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Blob {
    /// Object ID of the Sui object.
    pub id: ObjectID,
    /// The epoch in which the blob has been registered.
    pub registered_epoch: Epoch,
    /// The blob ID.
    #[serde_as(as = "DisplayFromStr")]
    pub blob_id: BlobId,
    /// The (unencoded) size of the blob.
    pub size: u64,
    /// The erasure coding type used for the blob.
    pub encoding_type: EncodingType,
    /// The epoch in which the blob was first certified, `None` if the blob is uncertified.
    pub certified_epoch: Option<Epoch>,
    /// The [`StorageResource`] used to store the blob.
    pub storage: StorageResource,
    /// Marks the blob as deletable.
    pub deletable: bool,
}

/// The output of the `store` command.
#[derive(Debug, Clone, Deserialize)]
#[allow(unused)]
pub struct StoreOutput(pub BlobStoreResult);

// Result when attempting to store a blob.
#[serde_as]
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DryRunOutput {
    pub storage_cost: u64,
}

/// The output of the `read` command.
#[serde_as]
#[derive(Debug, Clone, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
pub struct BlobIdOutput {
    #[serde_as(as = "DisplayFromStr")]
    pub blob_id: BlobId,
    pub file: PathBuf,
    pub unencoded_length: u64,
}

/// The output of the `info` command.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub(crate) struct InfoOutput {
    pub(crate) current_epoch: Epoch,
    pub(crate) n_shards: NonZeroU16,
    pub(crate) n_nodes: usize,
    pub(crate) storage_unit_size: u64,
    pub(crate) storage_price_per_unit_size: u64,
    pub(crate) write_price_per_unit_size: u64,
    pub(crate) max_blob_size: u64,
    pub(crate) marginal_size: u64,
    pub(crate) metadata_price: u64,
    pub(crate) marginal_price: u64,
    pub(crate) epoch_duration: Duration,
    pub(crate) max_epochs_ahead: u32,
    #[serde(skip_deserializing)]
    pub(crate) example_blobs: String,
    #[serde(skip_deserializing)]
    pub(crate) dev_info: String,
}

/// The number of shards, which can be deserialized from the output of the `info` command.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub(crate) struct NShards {
    pub(crate) n_shards: NonZeroU16,
}

pub fn try_from_output<T: DeserializeOwned>(output: Output) -> Result<T> {
    if !output.status.success() {
        // Format stderr as lossy utf8 to get the error print of the CLI.
        return Err(anyhow!(
            "running the command exited with error: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    };
    let output_str = String::from_utf8(output.stdout)?;
    serde_json::from_str(&output_str).context(format!(
        "failed to parse the Walrus CLI output: {output_str}"
    ))
}

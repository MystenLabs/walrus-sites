// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Types to interface with Walrus.

use core::fmt;
use std::{
    borrow::Cow,
    collections::BTreeMap,
    fmt::{Debug, Display},
    str::FromStr,
};

use base64::{display::Base64Display, engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The ID of a blob.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
#[repr(transparent)]
pub struct BlobId(pub [u8; Self::LENGTH]);

impl BlobId {
    /// The length of a blob ID in bytes.
    pub const LENGTH: usize = 32;
}

impl AsRef<[u8]> for BlobId {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Display for BlobId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Base64Display::new(self.as_ref(), &URL_SAFE_NO_PAD).fmt(f)
    }
}

impl Debug for BlobId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BlobId({self})")
    }
}

/// Error returned when unable to parse a blob ID.
#[derive(Debug, Error, PartialEq, Eq)]
#[error("failed to parse a blob ID")]
pub struct BlobIdParseError;

impl TryFrom<&'_ [u8]> for BlobId {
    type Error = BlobIdParseError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let bytes = <[u8; Self::LENGTH]>::try_from(value).map_err(|_| BlobIdParseError)?;
        Ok(Self(bytes))
    }
}

impl FromStr for BlobId {
    type Err = BlobIdParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut blob_id = Self([0; Self::LENGTH]);
        if let Ok(Self::LENGTH) = URL_SAFE_NO_PAD.decode_slice(s, &mut blob_id.0) {
            Ok(blob_id)
        } else {
            Err(BlobIdParseError)
        }
    }
}

/// Identifies a stored quilt patch.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredQuiltPatch {
    /// The identifier of the quilt patch.
    pub identifier: String,
    /// The quilt patch id.
    pub quilt_patch_id: String,
}

/// A enum wrapper around the quilt index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum QuiltIndex {
    /// QuiltIndexV1.
    V1(QuiltIndexV1),
}

/// An index over the [patches][QuiltPatchV1] (blobs) in a quilt.
///
/// Each quilt patch represents a blob stored in the quilt. And each patch is
/// mapped to a contiguous index range.
// INV: The patches are sorted by their end indices.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuiltIndexV1 {
    /// Location/identity index of the blob in the quilt.
    pub quilt_patches: Vec<QuiltPatchV1>,
}

/// Represents a blob within a unencoded quilt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuiltPatchV1 {
    /// The start sliver index of the blob.
    #[serde(skip)]
    pub start_index: u16,
    /// The end sliver index of the blob.
    pub end_index: u16,
    /// The identifier of the blob, it can be used to locate the blob in the quilt.
    pub identifier: String,
    /// The tags of the blob.
    //
    // A BTreeMap is used to ensure deterministic serialization.
    pub tags: BTreeMap<String, String>,
}

/// A wrapper around a blob and its identifier.
///
/// A valid identifier is a string that contains only alphanumeric characters,
/// underscores, hyphens, and periods.
#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
pub struct QuiltStoreBlob<'a> {
    /// The blob data, either borrowed or owned.
    blob: Cow<'a, [u8]>,
    /// The identifier of the blob.
    identifier: String,
    /// The tags of the blob.
    pub tags: BTreeMap<String, String>,
}

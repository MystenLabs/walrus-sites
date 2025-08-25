// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{
    fmt::{self, Display},
    path::{Path, PathBuf},
};

use anyhow::anyhow;
use move_core_types::u256::U256;

use crate::types::{HttpHeaders, LocalSuiResource};

/// The resource that is to be created or updated on Sui.
///
/// This struct contains additional information that is not stored on chain, compared to
/// [`SuiResource`] (`unencoded_size`, `full_path`).
///
/// [`Resource`] objects are always compared on their `info` field
/// ([`SuiResource`]), and never on their `unencoded_size` or `full_path`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct LocalResource {
    pub info: LocalSuiResource,
    /// The unencoded length of the resource.
    pub unencoded_size: usize,
    /// The full path of the resource on disk.
    pub full_path: PathBuf,
}

impl LocalResource {
    pub fn new(
        resource_path: String,
        full_path: PathBuf,
        headers: HttpHeaders,
        blob_hash: U256,
        unencoded_size: usize,
    ) -> Self {
        LocalResource {
            info: LocalSuiResource {
                path: resource_path,
                headers,
                blob_hash,
                // TODO(giac): eventually implement resource bundling.
                range: None,
            },
            unencoded_size,
            full_path,
        }
    }
}

impl Display for LocalResource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Resource: {:?}, sha256 hash: {})", // TODO(nikos): Check if we need to format to hex
            // ourselves
            self.info.path,
            self.info.blob_hash
        )
    }
}

pub(crate) fn full_path_to_resource_path(full_path: &Path, root: &Path) -> anyhow::Result<String> {
    let rel_path = full_path.strip_prefix(root)?;
    Ok(format!(
        "/{}",
        rel_path
            .to_str()
            .ok_or(anyhow!("could not process the path string: {:?}", rel_path))?
    ))
}

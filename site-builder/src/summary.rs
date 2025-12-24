// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Summaries of the run results.

use walrus_sdk::core::QuiltPatchId;

use crate::{
    site::{resource::SiteOps, SiteDataDiff},
    types::{ExtendOps, RouteOps},
    util::parse_quilt_patch_id,
    walrus::types::BlobId,
};

/// The struct can be turned into a summary.
pub trait Summarizable {
    fn to_summary(&self) -> String;
}

pub struct ResourceOpSummary {
    operation: String,
    path: String,
    blob_id: BlobId,
    quilt_patch_id: Option<QuiltPatchId>,
}

impl From<&SiteOps<'_>> for ResourceOpSummary {
    fn from(value: &SiteOps<'_>) -> Self {
        let (operation, info) = match value {
            SiteOps::Deleted(resource) => ("deleted".to_owned(), &resource.info),
            SiteOps::Created(resource) => ("created".to_owned(), &resource.info),
            SiteOps::Unchanged(resource) => ("unchanged".to_owned(), &resource.info),
            SiteOps::RemovedRoutes => {
                unreachable!("RemovedRoutes should not be converted into ResourceOpSummary")
            }
            SiteOps::BurnedSite => {
                unreachable!("BurnedSite should not be converted into ResourceOpSummary")
            }
        };
        let quilt_patch_id = parse_quilt_patch_id(&info.blob_id, &info.headers);
        ResourceOpSummary {
            operation,
            path: info.path.clone(),
            blob_id: info.blob_id,
            quilt_patch_id,
        }
    }
}

impl Summarizable for ResourceOpSummary {
    fn to_summary(&self) -> String {
        match &self.quilt_patch_id {
            Some(quilt_patch_id) => format!(
                "{} resource {} with quilt patch ID {}",
                self.operation, self.path, quilt_patch_id
            ),
            None => format!(
                "{} resource {} with blob ID {}",
                self.operation, self.path, self.blob_id
            ),
        }
    }
}

impl Summarizable for RouteOps {
    fn to_summary(&self) -> String {
        match self {
            RouteOps::Unchanged => "The site routes were left unchanged.".to_owned(),
            RouteOps::Replace(_) => "The site routes were modified.".to_owned(),
        }
    }
}

impl Summarizable for ExtendOps {
    fn to_summary(&self) -> String {
        match self {
            ExtendOps::Noop => "No blob extensions performed.".to_owned(),
            ExtendOps::Extend { blobs_epochs, .. } => {
                let lines: Vec<String> = blobs_epochs
                    .iter()
                    .map(|(obj_ref, epochs)| {
                        format!("  - Extended blob {} by {} epochs", obj_ref.0, epochs)
                    })
                    .collect();
                format!("Blob extensions:\n{}", lines.join("\n"))
            }
        }
    }
}

impl Summarizable for Vec<ResourceOpSummary> {
    fn to_summary(&self) -> String {
        self.iter()
            .map(|op| format!("  - {}", op.to_summary()))
            .collect::<Vec<_>>()
            .join("\n")
            .to_owned()
    }
}

pub struct SiteDataDiffSummary {
    pub resource_ops: Vec<ResourceOpSummary>,
    pub route_ops: RouteOps,
    pub metadata_updated: bool,
    pub site_name_updated: bool,
    pub extend_ops: ExtendOps,
}

impl From<&SiteDataDiff<'_>> for SiteDataDiffSummary {
    fn from(value: &SiteDataDiff<'_>) -> Self {
        SiteDataDiffSummary {
            resource_ops: value.resource_ops.iter().map(|op| op.into()).collect(),
            route_ops: value.route_ops.clone(),
            metadata_updated: !value.metadata_op.is_noop(),
            site_name_updated: !value.site_name_op.is_noop(),
            extend_ops: value.extend_ops.clone(),
        }
    }
}

impl From<SiteDataDiff<'_>> for SiteDataDiffSummary {
    fn from(value: SiteDataDiff<'_>) -> Self {
        (&value).into()
    }
}

impl Summarizable for SiteDataDiffSummary {
    fn to_summary(&self) -> String {
        if self.resource_ops.is_empty()
            && self.route_ops.is_unchanged()
            && !self.metadata_updated
            && !self.site_name_updated
            && self.extend_ops.is_noop()
        {
            return "No operation needs to be performed.".to_owned();
        }

        let resource_str = if !self.resource_ops.is_empty() {
            format!(
                "Resource operations performed:\n{}",
                // Update this so that if it's a quilt, use the quilt patch id
                self.resource_ops.to_summary()
            )
        } else {
            "No resource operations performed.".to_owned()
        };
        let route_str = self.route_ops.to_summary();
        let metadata_str = if self.metadata_updated {
            "Metadata updated."
        } else {
            "No Metadata updated."
        };
        let site_name_str = if self.site_name_updated {
            "The site name has been updated."
        } else {
            "Site name has not been updated."
        };
        let extend_str = self.extend_ops.to_summary();

        format!("{resource_str}\n{route_str}\n{metadata_str}\n{site_name_str}\n{extend_str}")
    }
}

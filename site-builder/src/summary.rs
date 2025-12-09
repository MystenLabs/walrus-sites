// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Summaries of the run results.

use walrus_core::QuiltPatchId;

use crate::{
    site::{resource::ResourceOp, SiteDataDiff},
    types::RouteOps,
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

impl From<&ResourceOp<'_>> for ResourceOpSummary {
    fn from(value: &ResourceOp<'_>) -> Self {
        let (operation, info) = match value {
            ResourceOp::Deleted(resource) => ("deleted".to_owned(), &resource.info),
            ResourceOp::Created(resource) => ("created".to_owned(), &resource.info),
            ResourceOp::Unchanged(resource) => ("unchanged".to_owned(), &resource.info),
            ResourceOp::RemovedRoutes => {
                unreachable!("RemovedRoutes should not be converted into ResourceOpSummary")
            }
            ResourceOp::BurnedSite => {
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
}

impl From<&SiteDataDiff<'_>> for SiteDataDiffSummary {
    fn from(value: &SiteDataDiff<'_>) -> Self {
        SiteDataDiffSummary {
            resource_ops: value.resource_ops.iter().map(|op| op.into()).collect(),
            route_ops: value.route_ops.clone(),
            metadata_updated: !value.metadata_op.is_noop(),
            site_name_updated: !value.site_name_op.is_noop(),
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

        format!("{resource_str}\n{route_str}\n{metadata_str}\n{site_name_str}")
    }
}

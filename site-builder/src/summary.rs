// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Summaries of the run results.

use crate::{
    site::{resource::ResourceOp, SiteDataDiff},
    types::RouteOps,
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
}

impl From<&ResourceOp<'_>> for ResourceOpSummary {
    fn from(value: &ResourceOp<'_>) -> Self {
        let (operation, info) = match value {
            ResourceOp::Deleted(resource) => ("deleted".to_owned(), &resource.info),
            ResourceOp::Created(resource) => ("created".to_owned(), &resource.info),
            ResourceOp::Unchanged(resource) => ("unchanged".to_owned(), &resource.info),
        };
        ResourceOpSummary {
            operation,
            path: info.path.clone(),
            blob_id: info.blob_id,
        }
    }
}

impl Summarizable for ResourceOpSummary {
    fn to_summary(&self) -> String {
        format!(
            "{} resource {} with blob ID {}",
            self.operation, self.path, self.blob_id
        )
    }
}

impl Summarizable for RouteOps {
    fn to_summary(&self) -> String {
        match self {
            RouteOps::Unchanged => "The site routes were left unchanged".to_owned(),
            RouteOps::Replace(_) => "The site routes were modified".to_owned(),
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
}

impl From<&SiteDataDiff<'_>> for SiteDataDiffSummary {
    fn from(value: &SiteDataDiff<'_>) -> Self {
        SiteDataDiffSummary {
            resource_ops: value.resource_ops.iter().map(|op| op.into()).collect(),
            route_ops: value.route_ops.clone(),
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
        if self.resource_ops.is_empty() && self.route_ops.is_unchanged() {
            return "No operation needs to be performed".to_owned();
        }

        let resource_str = if !self.resource_ops.is_empty() {
            format!(
                "Resource operations performed:\n{}\n",
                self.resource_ops.to_summary()
            )
        } else {
            "".to_owned()
        };
        let route_str = self.route_ops.to_summary();

        format!("{}{}", resource_str, route_str)
    }
}

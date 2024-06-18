// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use serde::Serialize;
use sui_types::{
    base_types::{ObjectID, SuiAddress},
    programmable_transaction_builder::ProgrammableTransactionBuilder,
    transaction::{Argument, CallArg, ProgrammableTransaction},
    Identifier,
    TypeTag,
};

use super::resource::{ResourceInfo, ResourceOp};

pub struct SitePtb<T = ()> {
    pt_builder: ProgrammableTransactionBuilder,
    site_argument: T,
    package: ObjectID,
    module: Identifier,
}

/// A PTB to update a site.
///
/// It is composed of a series of [`SiteCall`]s, which all have the Walrus site object id as
/// first argument.
impl SitePtb {
    pub fn new(package: ObjectID, module: Identifier) -> Result<Self> {
        let pt_builder = ProgrammableTransactionBuilder::new();
        Ok(SitePtb {
            pt_builder,
            site_argument: (),
            package,
            module,
        })
    }

    pub fn with_call_arg(mut self, site_arg: &CallArg) -> Result<SitePtb<Argument>> {
        let site_argument = self.pt_builder.input(site_arg.clone())?;
        Ok(SitePtb {
            pt_builder: self.pt_builder,
            site_argument,
            package: self.package,
            module: self.module,
        })
    }

    pub fn with_arg(self, site_arg: Argument) -> Result<SitePtb<Argument>> {
        Ok(SitePtb {
            pt_builder: self.pt_builder,
            site_argument: site_arg,
            package: self.package,
            module: self.module,
        })
    }

    /// Makes the call to create a new site and keeps the resulting argument.
    pub fn with_create_site(mut self, site_name: &str) -> Result<SitePtb<Argument>> {
        let argument = self.create_site(site_name)?;
        self.with_arg(argument)
    }
}

impl<T> SitePtb<T> {
    /// Transfer argument to address
    pub fn transfer_arg(&mut self, recipient: SuiAddress, arg: Argument) {
        self.pt_builder.transfer_arg(recipient, arg);
    }

    /// Move call to create a new Walrus site.
    pub fn create_site(&mut self, site_name: &str) -> Result<Argument> {
        tracing::debug!(site=%site_name, "new Move call: creating site");
        let name_arg = self.pt_builder.input(pure_call_arg(&site_name)?)?;
        Ok(self.add_programmable_move_call(Identifier::new("new_site")?, vec![], vec![name_arg]))
    }

    pub fn add_programmable_move_call(
        &mut self,
        function: Identifier,
        type_arguments: Vec<TypeTag>,
        call_args: Vec<Argument>,
    ) -> Argument {
        self.pt_builder.programmable_move_call(
            self.package,
            self.module.clone(),
            function,
            type_arguments,
            call_args,
        )
    }

    /// Concludes the creation of the PTB.
    pub fn finish(self) -> ProgrammableTransaction {
        self.pt_builder.finish()
    }
}

impl SitePtb<Argument> {
    pub fn site_argument(&self) -> Argument {
        self.site_argument
    }

    pub fn add_calls(&mut self, calls: impl IntoIterator<Item = SiteCall>) -> Result<()> {
        for call in calls {
            self.add_call(call)?;
        }
        Ok(())
    }

    /// Adds a call to the PTB.
    ///
    /// If the `function` field of the [`SiteCall`] provided is "new_resource_and_add", this
    /// function will create two transactions in the PTB, one to creat the resource, and one to add
    /// it to the site.
    pub fn add_call(&mut self, mut call: SiteCall) -> Result<()> {
        let mut args = call
            .args
            .into_iter()
            .map(|a| self.pt_builder.input(a))
            .collect::<Result<Vec<Argument>>>()?;

        if &call.function == "new_resource_and_add" {
            // This it the call to add a new resource to the ptb.
            // The first step is to create a new resource
            let new_resource_arg =
                self.add_programmable_move_call(Identifier::new("new_resource")?, vec![], args);
            args = vec![new_resource_arg];
            // Replace the call to execute the adding
            "add_resource".clone_into(&mut call.function);
        }
        args.insert(0, self.site_argument);
        self.add_programmable_move_call(Identifier::new(call.function)?, vec![], args);
        Ok(())
    }
}

// Testing out
#[derive(Debug)]
pub struct SiteCall {
    function: String,
    args: Vec<CallArg>,
}

impl SiteCall {
    /// Creates a new resource and adds it to the site.
    ///
    /// This call results into two transactions in a PTB, one to create the resource, and one to add
    /// it to the site.
    pub fn new_resource_and_add(resource: &ResourceInfo) -> Result<SiteCall> {
        tracing::debug!(
            resource=%resource.path,
            content_type=?resource.content_type,
            encoding=?resource.content_encoding,
            blob_id=?resource.blob_id,
            "new Move call: creating resource"
        );
        Ok(SiteCall {
            function: "new_resource_and_add".to_owned(),
            args: vec![
                pure_call_arg(&resource.path)?,
                pure_call_arg(&resource.content_type.to_string())?,
                pure_call_arg(&resource.content_encoding.to_string())?,
                pure_call_arg(&resource.blob_id)?,
            ],
        })
    }

    /// Removes a resource from the site if it exists
    pub fn remove_resource_if_exists(resource: &ResourceInfo) -> Result<SiteCall> {
        tracing::debug!(resource=%resource.path, "new Move call: removing resource");
        Ok(SiteCall {
            function: "remove_resource_if_exists".to_owned(),
            args: vec![pure_call_arg(&resource.path)?],
        })
    }
}

pub fn pure_call_arg<T: Serialize>(arg: &T) -> Result<CallArg> {
    Ok(CallArg::Pure(bcs::to_bytes(arg)?))
}

impl<'a> TryFrom<&ResourceOp<'a>> for SiteCall {
    type Error = anyhow::Error;

    fn try_from(value: &ResourceOp) -> Result<Self, Self::Error> {
        match value {
            ResourceOp::Deleted(resource) => SiteCall::remove_resource_if_exists(&resource.info),
            ResourceOp::Created(resource) => SiteCall::new_resource_and_add(&resource.info),
        }
    }
}

impl<'a> TryFrom<ResourceOp<'a>> for SiteCall {
    type Error = anyhow::Error;

    fn try_from(value: ResourceOp) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

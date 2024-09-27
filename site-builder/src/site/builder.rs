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

use super::resource::{HttpHeader, Resource, ResourceOp};

pub struct SitePtb<T = ()> {
    pt_builder: ProgrammableTransactionBuilder,
    site_argument: T,
    package: ObjectID,
    module: Identifier,
}

/// A PTB to update a site.
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
    fn transfer_arg(&mut self, recipient: SuiAddress, arg: Argument) {
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
    pub fn add_operations<'a>(
        &mut self,
        calls: impl IntoIterator<Item = &'a ResourceOp<'a>>,
    ) -> Result<()> {
        for call in calls {
            match call {
                ResourceOp::Deleted(resource) => self.remove_resource_if_exists(resource)?,
                ResourceOp::Created(resource) => self.add_resource(resource)?,
            }
        }
        Ok(())
    }

    pub fn transfer_site(&mut self, recipient: SuiAddress) {
        self.transfer_arg(recipient, self.site_argument);
    }

    /// Adds the move calls to remove a resource from the site, if the resource exists.
    pub fn remove_resource_if_exists(&mut self, resource: &Resource) -> Result<()> {
        tracing::debug!(resource=%resource.info.path, "new Move call: removing resource");
        let path_input = self.pt_builder.input(pure_call_arg(&resource.info.path)?)?;
        self.add_programmable_move_call(
            Identifier::new("remove_resource_if_exists")?,
            vec![],
            vec![self.site_argument, path_input],
        );
        Ok(())
    }

    /// Adds the move calls to create and add a resource to the site, with the specified headers.
    pub fn add_resource(&mut self, resource: &Resource) -> Result<()> {
        tracing::debug!(resource=%resource.info.path, "new Move call: adding resource");
        let new_resource_arg = self.create_resource(resource)?;

        // Add the headers to the resource.
        for header in resource.info.headers.0.iter() {
            self.add_header(new_resource_arg, header)?;
        }

        // Add the resource to the site.
        self.add_programmable_move_call(
            Identifier::new("add_resource")?,
            vec![],
            vec![self.site_argument, new_resource_arg],
        );

        Ok(())
    }

    /// Adds the move calls to create a resource.
    ///
    /// Returns the [`Argument`] for the newly-created resource.
    fn create_resource(&mut self, resource: &Resource) -> Result<Argument> {
        let inputs = [
            pure_call_arg(&resource.info.path)?,
            pure_call_arg(&resource.info.blob_id)?,
            pure_call_arg(&resource.info.blob_hash)?,
        ]
        .into_iter()
        .map(|arg| self.pt_builder.input(arg))
        .collect::<Result<Vec<_>>>()?;

        Ok(self.add_programmable_move_call(Identifier::new("new_resource")?, vec![], inputs))
    }

    /// Adds the header to the given resource argument.
    fn add_header(&mut self, resource_arg: Argument, header: &HttpHeader) -> Result<()> {
        let name_input = self.pt_builder.input(pure_call_arg(&header.name)?)?;
        let value_input = self.pt_builder.input(pure_call_arg(&header.value)?)?;
        self.add_programmable_move_call(
            Identifier::new("add_header")?,
            vec![],
            vec![resource_arg, name_input, value_input],
        );
        Ok(())
    }
}

pub fn pure_call_arg<T: Serialize>(arg: &T) -> Result<CallArg> {
    Ok(CallArg::Pure(bcs::to_bytes(arg)?))
}

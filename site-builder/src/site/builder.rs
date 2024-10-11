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

use super::{
    contracts::FunctionTag,
    resource::{Resource, ResourceOp},
    RouteOps,
};
use crate::{site::contracts, types::Range};

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
        Ok(self.add_programmable_move_call(
            contracts::site::new_site.identifier(),
            vec![],
            vec![name_arg],
        ))
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
    pub fn add_resource_operations<'a>(
        &mut self,
        calls: impl IntoIterator<Item = &'a ResourceOp<'a>>,
    ) -> Result<()> {
        for call in calls {
            match call {
                ResourceOp::Deleted(resource) => self.remove_resource_if_exists(resource)?,
                ResourceOp::Created(resource) => self.add_resource(resource)?,
                ResourceOp::Unchanged(_) => (),
            }
        }
        Ok(())
    }

    /// Adds move calls to update the routes on the object.
    pub fn add_route_operations(&mut self, route_ops: &RouteOps) -> Result<()> {
        if let RouteOps::Replace(new_routes) = route_ops {
            self.remove_routes()?;
            if !new_routes.is_empty() {
                self.create_routes()?;
                for (name, value) in new_routes.0.iter() {
                    self.add_route(name, value)?;
                }
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
            contracts::site::remove_resource_if_exists.identifier(),
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
            self.add_header(new_resource_arg, header.0, header.1)?;
        }

        // Add the resource to the site.
        self.add_programmable_move_call(
            contracts::site::add_resource.identifier(),
            vec![],
            vec![self.site_argument, new_resource_arg],
        );

        Ok(())
    }

    /// Adds the move calls to create a resource.
    ///
    /// Returns the [`Argument`] for the newly-created resource.
    fn create_resource(&mut self, resource: &Resource) -> Result<Argument> {
        let new_range_arg = self.create_range(&resource.info.range)?;

        let mut inputs = [
            pure_call_arg(&resource.info.path)?,
            pure_call_arg(&resource.info.blob_id)?,
            pure_call_arg(&resource.info.blob_hash)?,
        ]
        .into_iter()
        .map(|arg| self.pt_builder.input(arg))
        .collect::<Result<Vec<_>>>()?;

        inputs.push(new_range_arg);

        Ok(self.add_programmable_move_call(
            contracts::site::new_resource.identifier(),
            vec![],
            inputs,
        ))
    }

    fn create_range(&mut self, range: &Option<Range>) -> Result<Argument> {
        let inputs = [
            pure_call_arg(&range.as_ref().and_then(|r| r.start))?,
            pure_call_arg(&range.as_ref().and_then(|r| r.end))?,
        ]
        .into_iter()
        .map(|arg| self.pt_builder.input(arg))
        .collect::<Result<Vec<_>>>()?;

        Ok(self.add_programmable_move_call(
            contracts::site::new_range_option.identifier(),
            vec![],
            inputs,
        ))
    }

    /// Adds the header to the given resource argument.
    fn add_header(&mut self, resource_arg: Argument, name: &str, value: &str) -> Result<()> {
        self.add_key_value_to_argument(contracts::site::add_header, resource_arg, name, value)
    }

    /// Adds the move calls to add key and value to the argument.
    fn add_key_value_to_argument(
        &mut self,
        fn_name: FunctionTag,
        arg: Argument,
        key: &str,
        value: &str,
    ) -> Result<()> {
        let name_input = self.pt_builder.input(pure_call_arg(&key.to_owned())?)?;
        let value_input = self.pt_builder.input(pure_call_arg(&value.to_owned())?)?;
        self.add_programmable_move_call(
            fn_name.identifier(),
            vec![],
            vec![arg, name_input, value_input],
        );
        Ok(())
    }

    // Routes

    /// Adds the move calls to create a new routes object.
    fn create_routes(&mut self) -> Result<()> {
        self.add_programmable_move_call(
            contracts::site::create_routes.identifier(),
            vec![],
            vec![self.site_argument],
        );
        Ok(())
    }

    /// Adds the move calls to remove the routes object.
    fn remove_routes(&mut self) -> Result<()> {
        self.add_programmable_move_call(
            contracts::site::remove_all_routes_if_exist.identifier(),
            vec![],
            vec![self.site_argument],
        );
        Ok(())
    }

    /// Adds the move calls add a route to the routes object.
    fn add_route(&mut self, name: &str, value: &str) -> Result<()> {
        tracing::debug!(name=%name, value=%value, "new Move call: adding route");
        self.add_key_value_to_argument(
            contracts::site::insert_route,
            self.site_argument,
            name,
            value,
        )
    }
}

pub fn pure_call_arg<T: Serialize>(arg: &T) -> Result<CallArg> {
    Ok(CallArg::Pure(bcs::to_bytes(arg)?))
}

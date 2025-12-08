// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::collections::btree_map;

use anyhow::Result;
use serde::Serialize;
use sui_types::{
    base_types::{ObjectID, SuiAddress},
    programmable_transaction_builder::ProgrammableTransactionBuilder,
    transaction::{Argument, CallArg, ProgrammableTransaction},
    Identifier,
    TypeTag,
};
use thiserror::Error;

use super::{
    contracts::FunctionTag,
    resource::{Resource, ResourceOp},
};
use crate::{
    site::contracts,
    types::{Metadata, Range},
};

#[cfg(test)]
#[path = "../unit_tests/site.builder.tests.rs"]
mod site_builder_tests;

// We limit the max-move-calls to 1000 to protect also against max-dynamic-field-accesses per PTB,
// triggered from `remove_resource_if_exists`
// TODO?: Track multiple limits and behave accordingly.
pub const PTB_MAX_MOVE_CALLS: u16 = 1000;

/// Error type to differentiate max-move-calls limit reached from other unexpected `anyhow` errors.
#[derive(Debug, Error)]
pub enum SitePtbBuilderError {
    #[error("Exceeded maximum number of move-calls ({0}) in Transaction")]
    TooManyMoveCalls(u16),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type SitePtbBuilderResult<T> = Result<T, SitePtbBuilderError>;
pub trait SitePtbBuilderResultExt<T> {
    /// Ignores `TooManyMoveCalls` errors, propagates `Other` errors
    fn ok_if_limit_reached(self) -> Result<Option<T>>;
}

impl<T> SitePtbBuilderResultExt<T> for SitePtbBuilderResult<T> {
    fn ok_if_limit_reached(self) -> Result<Option<T>> {
        match self {
            Ok(val) => Ok(Some(val)),
            Err(SitePtbBuilderError::TooManyMoveCalls(_)) => Ok(None),
            Err(SitePtbBuilderError::Other(e)) => Err(e),
        }
    }
}

pub struct SitePtb<T = (), const MAX_MOVE_CALLS: u16 = PTB_MAX_MOVE_CALLS> {
    pt_builder: ProgrammableTransactionBuilder,
    move_call_counter: u16,
    site_argument: T,
    package: ObjectID,
    module: Identifier,
}

/// A PTB to update a site.
impl<const MAX_MOVE_CALLS: u16> SitePtb<(), MAX_MOVE_CALLS> {
    pub fn new(package: ObjectID, module: Identifier) -> Self {
        let pt_builder = ProgrammableTransactionBuilder::new();
        SitePtb {
            pt_builder,
            move_call_counter: 0,
            site_argument: (),
            package,
            module,
        }
    }

    pub fn with_call_arg(self, site_arg: &CallArg) -> Result<SitePtb<Argument, MAX_MOVE_CALLS>> {
        let Self {
            mut pt_builder,
            move_call_counter,
            package,
            module,
            ..
        } = self;
        let site_argument = pt_builder.input(site_arg.clone())?;
        Ok(SitePtb {
            pt_builder,
            move_call_counter,
            site_argument,
            package,
            module,
        })
    }

    pub fn with_arg(self, site_argument: Argument) -> SitePtb<Argument, MAX_MOVE_CALLS> {
        let Self {
            pt_builder,
            move_call_counter,
            package,
            module,
            ..
        } = self;
        SitePtb {
            pt_builder,
            move_call_counter,
            site_argument,
            package,
            module,
        }
    }

    /// Makes the call to create a new site and keeps the resulting argument.
    pub fn with_create_site(
        mut self,
        site_name: &str,
        metadata: Option<Metadata>,
    ) -> SitePtbBuilderResult<SitePtb<Argument, MAX_MOVE_CALLS>> {
        let argument = self.create_site(site_name, metadata)?;
        Ok(self.with_arg(argument))
    }
}

impl<T, const MAX_MOVE_CALLS: u16> SitePtb<T, MAX_MOVE_CALLS> {
    /// Transfer argument to address
    fn transfer_arg(&mut self, recipient: SuiAddress, arg: Argument) -> SitePtbBuilderResult<()> {
        self.increment_counter()?;
        self.pt_builder.transfer_arg(recipient, arg);
        Ok(())
    }

    /// Move call to create a new Walrus site.
    pub fn create_site(
        &mut self,
        site_name: &str,
        metadata: Option<Metadata>,
    ) -> SitePtbBuilderResult<Argument> {
        tracing::debug!(site=%site_name, "new Move call: creating site");
        // Needs metadata and site calls to happen atomically, one cannot happen without the other.
        self.check_counter_in_advance(2)?; // create metadata + site

        let name_arg = self.pt_builder.input(pure_call_arg(&site_name)?)?;
        let metadata_arg = match metadata {
            Some(metadata) => self.new_metadata(metadata),
            None => self.new_metadata(Metadata::default()),
        }?;
        self.add_programmable_move_call(
            contracts::site::new_site.identifier(),
            vec![],
            vec![name_arg, metadata_arg],
        )
    }

    fn new_metadata(&mut self, metadata: Metadata) -> SitePtbBuilderResult<Argument> {
        let defaults = Metadata::default();
        let args = [
            metadata.link.or(defaults.link),
            metadata.image_url.or(defaults.image_url),
            metadata.description.or(defaults.description),
            metadata.project_url.or(defaults.project_url),
            metadata.creator.or(defaults.creator),
        ]
        .into_iter()
        .map(|val| self.pt_builder.pure(val))
        .collect::<anyhow::Result<Vec<_>>>()?;

        self.increment_counter()?;
        Ok(self.pt_builder.programmable_move_call(
            self.package,
            Identifier::new("metadata").unwrap(),
            Identifier::new("new_metadata").unwrap(),
            vec![],
            args,
        ))
    }

    fn add_programmable_move_call(
        &mut self,
        function: Identifier,
        type_arguments: Vec<TypeTag>,
        call_args: Vec<Argument>,
    ) -> SitePtbBuilderResult<Argument> {
        self.increment_counter()?;
        Ok(self.pt_builder.programmable_move_call(
            self.package,
            self.module.clone(),
            function,
            type_arguments,
            call_args,
        ))
    }

    /// Concludes the creation of the PTB.
    pub fn finish(self) -> ProgrammableTransaction {
        self.pt_builder.finish()
    }

    pub fn with_max_move_calls<const NEW_MAX: u16>(self) -> SitePtb<T, NEW_MAX> {
        // Optimally we would use a static_assertions::const_assert here, but it needs unstable
        // feature: `#![feature(generic_const_exprs)]` to use it with generic parameters.
        debug_assert!(NEW_MAX <= PTB_MAX_MOVE_CALLS);
        let Self {
            pt_builder,
            move_call_counter,
            site_argument,
            package,
            module,
        } = self;
        SitePtb {
            pt_builder,
            move_call_counter,
            site_argument,
            package,
            module,
        }
    }

    fn check_counter_in_advance(&self, move_calls_needed: u16) -> Result<(), SitePtbBuilderError> {
        match move_calls_needed + self.move_call_counter {
            c if c > MAX_MOVE_CALLS => Err(SitePtbBuilderError::TooManyMoveCalls(MAX_MOVE_CALLS)),
            _ => Ok(()),
        }
    }

    fn increment_counter(&mut self) -> SitePtbBuilderResult<()> {
        if self.move_call_counter + 1 > MAX_MOVE_CALLS {
            return Err(SitePtbBuilderError::TooManyMoveCalls(MAX_MOVE_CALLS));
        }
        self.move_call_counter += 1;
        Ok(())
    }
}

impl<const MAX_MOVE_CALLS: u16> SitePtb<Argument, MAX_MOVE_CALLS> {
    pub fn add_resource_operations<'a>(
        &mut self,
        calls: &mut std::iter::Peekable<impl Iterator<Item = &'a ResourceOp<'a>>>,
    ) -> SitePtbBuilderResult<()> {
        while let Some(call) = calls.peek() {
            match call {
                ResourceOp::Deleted(resource) => self.remove_resource_if_exists(resource)?,
                ResourceOp::Created(resource) => self.add_resource(resource)?,
                ResourceOp::Unchanged(_) => (),
            }
            calls.next();
        }
        Ok(())
    }

    /// Adds move calls to update the routes on the object.
    pub fn add_route_operations(
        &mut self,
        new_routes_iter: &mut std::iter::Peekable<btree_map::Iter<String, String>>,
    ) -> SitePtbBuilderResult<()> {
        while let Some((name, value)) = new_routes_iter.peek() {
            self.add_route(name, value)?;
            new_routes_iter.next();
        }
        Ok(())
    }

    pub fn with_update_metadata(
        mut self,
        metadata: Metadata,
    ) -> SitePtbBuilderResult<SitePtb<Argument, MAX_MOVE_CALLS>> {
        let metadata = self.new_metadata(metadata)?;
        self.add_programmable_move_call(
            contracts::site::update_metadata.identifier(),
            vec![],
            vec![self.site_argument, metadata],
        )?;
        Ok(self)
    }

    pub fn transfer_site(&mut self, recipient: SuiAddress) -> SitePtbBuilderResult<()> {
        self.transfer_arg(recipient, self.site_argument)
    }

    /// Adds the move calls to remove a resource from the site, if the resource exists.
    pub fn remove_resource_if_exists(&mut self, resource: &Resource) -> SitePtbBuilderResult<()> {
        tracing::debug!(resource=%resource.info.path, "new Move call: removing resource");
        let path_input = self.pt_builder.input(pure_call_arg(&resource.info.path)?)?;
        self.add_programmable_move_call(
            contracts::site::remove_resource_if_exists.identifier(),
            vec![],
            vec![self.site_argument, path_input],
        )?;
        Ok(())
    }

    /// Adds the move calls to create and add a resource to the site, with the specified headers.
    pub fn add_resource(&mut self, resource: &Resource) -> SitePtbBuilderResult<()> {
        // Header insertions in resource can currently happen only atomically. We need to be
        // certain that the `for header` loop will end without exceeding max-move-calls.
        let headers_count = resource.info.headers.len() as u16;
        let move_calls_needed = headers_count + 2; // create resource + add df
        if move_calls_needed > PTB_MAX_MOVE_CALLS {
            // We would need to half-store a resource at the end of the PTB, and use:
            // `@walrus/sites::site::{remove_resource -> add_header x n -> add_resource}` in the
            // next PTBs
            return Err(anyhow::anyhow!(
                "Cannot handle these many ({headers_count}) headers in resource"
            )
            .into());
        };
        self.check_counter_in_advance(move_calls_needed)?;

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
        )?;

        Ok(())
    }

    /// Removes all dynamic fields and then burns the site.
    /// Returns true when the limit reached and a new ptb should be 
    /// used. 
    pub fn destroy<'a, I>(
        &mut self,
        resources: &mut I,
    ) -> SitePtbBuilderResult<bool>
    where
        I: Iterator<Item = &'a Resource>,
    {
        for resource in resources.by_ref() {
            if self
                .remove_resource_if_exists(resource)
                .ok_if_limit_reached()?
                .is_none()
            {
                // Move call limit reached, iterator position is preserved
                return Ok(true);
            }
        }
        // All resources processed
        Ok(false)
    }

    /// Adds the move calls to create a resource.
    ///
    /// Returns the [`Argument`] for the newly-created resource.
    fn create_resource(&mut self, resource: &Resource) -> SitePtbBuilderResult<Argument> {
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

        self.add_programmable_move_call(contracts::site::new_resource.identifier(), vec![], inputs)
    }

    fn create_range(&mut self, range: &Option<Range>) -> SitePtbBuilderResult<Argument> {
        let inputs = [
            pure_call_arg(&range.as_ref().and_then(|r| r.start))?,
            pure_call_arg(&range.as_ref().and_then(|r| r.end))?,
        ]
        .into_iter()
        .map(|arg| self.pt_builder.input(arg))
        .collect::<Result<Vec<_>>>()?;

        self.add_programmable_move_call(
            contracts::site::new_range_option.identifier(),
            vec![],
            inputs,
        )
    }

    /// Adds the header to the given resource argument.
    fn add_header(
        &mut self,
        resource_arg: Argument,
        name: &str,
        value: &str,
    ) -> SitePtbBuilderResult<()> {
        self.add_key_value_to_argument(contracts::site::add_header, resource_arg, name, value)
    }

    /// Adds the move calls to add key and value to the argument.
    fn add_key_value_to_argument(
        &mut self,
        fn_name: FunctionTag,
        arg: Argument,
        key: &str,
        value: &str,
    ) -> SitePtbBuilderResult<()> {
        let name_input = self.pt_builder.input(pure_call_arg(&key.to_owned())?)?;
        let value_input = self.pt_builder.input(pure_call_arg(&value.to_owned())?)?;
        self.add_programmable_move_call(
            fn_name.identifier(),
            vec![],
            vec![arg, name_input, value_input],
        )?;
        Ok(())
    }

    // Routes

    /// Adds the move calls to create a new routes object.
    fn create_routes(&mut self) -> SitePtbBuilderResult<()> {
        self.add_programmable_move_call(
            contracts::site::create_routes.identifier(),
            vec![],
            vec![self.site_argument],
        )?;
        Ok(())
    }

    /// Adds the move calls to remove and create new routes
    pub fn replace_routes(&mut self) -> SitePtbBuilderResult<()> {
        self.check_counter_in_advance(2)?; // remove + create routes
        self.remove_routes()?;
        self.create_routes()?;
        Ok(())
    }

    /// Adds the move calls to remove the routes object.
    // TODO: Remove pub and move RouteOp logic from `manager.rs`?
    pub fn remove_routes(&mut self) -> SitePtbBuilderResult<()> {
        self.check_counter_in_advance(1)?;
        self.add_programmable_move_call(
            contracts::site::remove_all_routes_if_exist.identifier(),
            vec![],
            vec![self.site_argument],
        )?;
        Ok(())
    }

    /// Adds the move calls add a route to the routes object.
    fn add_route(&mut self, name: &str, value: &str) -> SitePtbBuilderResult<()> {
        tracing::debug!(name=%name, value=%value, "new Move call: adding route");
        self.add_key_value_to_argument(
            contracts::site::insert_route,
            self.site_argument,
            name,
            value,
        )
    }

    pub fn update_name(&mut self, name: &str) -> SitePtbBuilderResult<()> {
        tracing::debug!(name=%name, "new Move call: updating site name");
        let name_input = self.pt_builder.input(pure_call_arg(&name.to_owned())?)?;
        self.add_programmable_move_call(
            contracts::site::update_name.identifier(),
            vec![],
            vec![self.site_argument, name_input],
        )?;
        Ok(())
    }

    /// Burns the site.
    pub fn burn(&mut self) -> SitePtbBuilderResult<()> {
        self.check_counter_in_advance(1)?;
        self.add_programmable_move_call(
            contracts::site::burn.identifier(),
            vec![],
            vec![self.site_argument],
        )?;
        Ok(())
    }
}

pub fn pure_call_arg<T: Serialize>(arg: &T) -> Result<CallArg> {
    Ok(CallArg::Pure(bcs::to_bytes(arg)?))
}

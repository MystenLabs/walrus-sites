// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::collections::btree_map;

use anyhow::{Context, Result};
use serde::Serialize;
use sui_types::{
    base_types::{ObjectID, ObjectRef, SuiAddress},
    object::{Object, Owner},
    programmable_transaction_builder::ProgrammableTransactionBuilder,
    transaction::{
        Argument,
        CallArg,
        Command,
        ObjectArg,
        ProgrammableTransaction,
        SharedObjectMutability,
    },
    Identifier,
    TypeTag,
};
use thiserror::Error;
use walrus_sui::coin::Coin;

use super::{
    contracts::FunctionTag,
    resource::{Resource, SiteOps},
};
use crate::{
    site::contracts,
    types::{Metadata, Range, Redirect},
};

#[cfg(test)]
#[path = "../unit_tests/site.builder.tests.rs"]
mod site_builder_tests;

/// Maximum move calls per PTB.
/// We limit the max-move-calls to 1000 to protect also against max-dynamic-field-accesses per PTB,
/// triggered from `remove_resource_if_exists`.
pub const PTB_MAX_MOVE_CALLS: u16 = 1000;

/// Estimated byte-size threshold for PTB payload data (key/value strings, resource paths, blob
/// IDs, and per-command overhead). When `bytes_estimate` exceeds this, the PTB is considered
/// full. This is a secondary guard alongside the move-call limit.
/// Note: We do not track things that take up "constant" size in the PTB, like the site-argument,
/// the routes df replacement, metadata replacement etc.
// owned-object: 74 bytes
// pure argument adds 2 bytes eg. u64: 10 bytes
// move-call: ~73 + 3 x (arg) bytes -> 100 bytes
pub const PTB_MAX_BYTES: usize = 120_000;

/// BCS size of a `Command::MoveCall` excluding argument references: enum tag (1) + package
/// ObjectID (32) + module string (~16) + function string (~21) + empty type_arguments vec (1) +
/// arguments vec length prefix (1). Rounds up to 100 to err on the high side.
/// Input sizes (owned objects, pure args) are tracked separately.
const PTB_MOVE_CALL_SIZE: usize = 100;

/// BCS size of a single `Argument` reference inside a MoveCall's arguments vector:
/// enum variant tag (1) + u16 index (2) = 3 bytes.
const PTB_ARGUMENT_REF_SIZE: usize = 3;

/// BCS size of an `extend_blob` operation: owned object input (74) + u32 epochs pure arg (6) +
/// move call (~100) + overhead, rounded up to 200.
const PTB_EXTEND_OPERATION_SIZE: usize = 200;

/// BCS overhead for a `String` pure argument: CallArg enum tag (1) + Pure `Vec<u8>` length (1-2) +
/// BCS string length prefix (1-2) + padding. Rounded up to 10. The total size of a string input is
/// `string.len() + PTB_STRING_PURE_ARG_OVERHEAD`.
const PTB_STRING_PURE_ARG_OVERHEAD: usize = 10;

/// BCS size of a `u256` pure argument: CallArg enum tag (1) + Pure `Vec<u8>` length (1) +
/// 32 bytes payload = 34. Rounded up to 40.
const PTB_U256_PURE_ARG_SIZE: usize = 40;

/// BCS size of a `u16` pure argument: CallArg enum tag (1) + Pure `Vec<u8>` length (1) +
/// 2 bytes payload = 4. Rounded up to 6.
const PTB_U16_PURE_ARG_SIZE: usize = 6;

/// BCS size of an `Option<Range>` pure input when both fields are `None`: two `Option::None`
/// pure args, each 3 bytes (CallArg tag (1) + Pure vec len (1) + ULEB128 length 0 (1)).
const PTB_RANGE_NONE_SIZE: usize = 6;

/// BCS size of an `Option<Range>` pure input when both fields are `Some(u64)`: two
/// `Option::Some(u64)` pure args, each 11 bytes (CallArg tag (1) + Pure vec len (1) +
/// ULEB128 length 1 (1) + u64 (8)).
const PTB_RANGE_SOME_SIZE: usize = 22;

trait HasIdentifier {
    fn identifier(&self) -> Identifier;
}

impl HasIdentifier for FunctionTag<'_> {
    fn identifier(&self) -> Identifier {
        Identifier::new(self.name).expect("function name is a valid identifier")
    }
}

/// Error type to differentiate max-move-calls limit reached from other unexpected `anyhow` errors.
#[derive(Debug, Error)]
pub enum SitePtbBuilderError {
    #[error("Exceeded maximum number of move-calls ({0}) in Transaction")]
    TooManyMoveCalls(u16),
    #[error("Estimated Transaction size ({0} bytes) exceeds maximum ({PTB_MAX_BYTES})")]
    PtbSizeEstimationExceeded(usize),
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
            Err(SitePtbBuilderError::PtbSizeEstimationExceeded(_)) => Ok(None),
            Err(SitePtbBuilderError::Other(e)) => Err(e),
        }
    }
}

pub struct SitePtb<T = (), const MAX_MOVE_CALLS: u16 = PTB_MAX_MOVE_CALLS> {
    pt_builder: ProgrammableTransactionBuilder,
    move_call_counter: u16,
    /// Erroring on the very high side.
    bytes_estimate: usize,
    site_argument: T,
    package: ObjectID,
    module: Identifier,
    walrus_package: ObjectID,
    system_obj_arg: Option<Argument>,
    wal_coin_arg: Option<Argument>,
}

// TBD: Now that we also have multiple limits, does it make sense to have `MAX_MOVE_CALLS` const
// generic?
// Arguments for yes:
// - MAX_MOVE_CALLS is changed during one ptb to make sure some calls are inserted in one eg.
// transfer
// Arguments for no:
// - It can be easily converted into a field inside the SitePtb and probably remove some code?
/// A PTB to update a site.
impl<const MAX_MOVE_CALLS: u16> SitePtb<(), MAX_MOVE_CALLS> {
    pub fn new(package: ObjectID, module: Identifier, walrus_package: ObjectID) -> Self {
        let pt_builder = ProgrammableTransactionBuilder::new();
        SitePtb {
            pt_builder,
            move_call_counter: 0,
            bytes_estimate: 0,
            site_argument: (),
            package,
            module,
            walrus_package,
            system_obj_arg: None,
            wal_coin_arg: None,
        }
    }

    pub fn with_call_arg(self, site_arg: &CallArg) -> Result<SitePtb<Argument, MAX_MOVE_CALLS>> {
        let Self {
            mut pt_builder,
            move_call_counter,
            bytes_estimate,
            package,
            module,
            walrus_package,
            system_obj_arg,
            wal_coin_arg,
            ..
        } = self;
        let site_argument = pt_builder.input(site_arg.clone())?;
        Ok(SitePtb {
            pt_builder,
            move_call_counter,
            bytes_estimate,
            site_argument,
            package,
            module,
            walrus_package,
            system_obj_arg,
            wal_coin_arg,
        })
    }

    pub fn with_arg(self, site_argument: Argument) -> SitePtb<Argument, MAX_MOVE_CALLS> {
        let Self {
            pt_builder,
            move_call_counter,
            bytes_estimate,
            package,
            module,
            walrus_package,
            system_obj_arg,
            wal_coin_arg,
            ..
        } = self;
        SitePtb {
            pt_builder,
            move_call_counter,
            bytes_estimate,
            site_argument,
            package,
            module,
            walrus_package,
            system_obj_arg,
            wal_coin_arg,
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
    /// Transfer argument to address.
    ///
    /// In our case, transfer_arg is only used for the Site argument. If we cannot transfer the
    /// newly created site, the transaction will abort. So, we error "hard" by not
    /// returning `SitePtbBuilderError::TooManyMoveCalls`, and instead an anyhow::Error, so that it
    /// cannot be skipped by `ok_if_limit_reached()`.
    fn transfer_arg(&mut self, recipient: SuiAddress, arg: Argument) -> SitePtbBuilderResult<()> {
        self.increment_move_call_counter()
            .context("Could not fit transfer_arg into the PTB.")?;
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
        self.move_call_check(2)?; // create metadata + site

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

    pub fn fill_walrus_system_and_coin(
        &mut self,
        coins: Vec<Coin>,
        system_obj: Object,
    ) -> SitePtbBuilderResult<()> {
        if self.system_obj_arg.is_some() {
            return Err(anyhow::anyhow!("Tried to set walrus System argument twice.").into());
        }
        if self.wal_coin_arg.is_some() {
            return Err(anyhow::anyhow!("Tried to set WAL coin argument twice.").into());
        }

        let system_arg = self.extract_system_arg(system_obj)?;
        self.system_obj_arg.replace(system_arg);
        let wal_coin_arg = self.create_wal_coin(coins)?;
        self.wal_coin_arg.replace(wal_coin_arg);
        Ok(())
    }

    // TODO: We should be handling extend operations similarly to resources and routes, and be able
    // to split them between 2 ptbs.
    // Due to Quilts, the case where we have a large number of extend-operations shouldn't be that
    // high, HOWEVER, we cannot be sure how people use this tool, so better to enable this
    // (splitting to multiple ptbs).
    pub fn add_extend_operations(
        &mut self,
        blobs_to_extend: impl IntoIterator<Item = (ObjectRef, u32), IntoIter: ExactSizeIterator>,
    ) -> SitePtbBuilderResult<()> {
        let blobs_to_extend = blobs_to_extend.into_iter();
        let count = blobs_to_extend.len();
        // Check limits in advance as we currently do not support splitting extend operations
        // between ptbs.
        self.limits_check(count as u16, count * PTB_EXTEND_OPERATION_SIZE)?;
        for (blob_ref, epochs) in blobs_to_extend {
            self.extend_blob(blob_ref, epochs)?;
        }
        Ok(())
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

        self.increment_move_call_counter()?;
        Ok(self.pt_builder.programmable_move_call(
            self.package,
            Identifier::new("metadata").unwrap(),
            Identifier::new("new_metadata").unwrap(),
            vec![],
            args,
        ))
    }

    // TODO: Every call of this does not use type_arguments. Let's remove it
    fn add_programmable_move_call(
        &mut self,
        function: Identifier,
        type_arguments: Vec<TypeTag>,
        call_args: Vec<Argument>,
    ) -> SitePtbBuilderResult<Argument> {
        // Each move call adds ~73 + 3 x (arg) bytes -> 100 bytes erroring on the high side.
        // (package ID + module + function + args encoding - BCS serialization of inputs/results
        // and object references are measured separately).
        self.limits_check_and_update(
            1,
            PTB_MOVE_CALL_SIZE + call_args.len() * PTB_ARGUMENT_REF_SIZE,
        )?;
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
            bytes_estimate,
            site_argument,
            package,
            module,
            walrus_package,
            system_obj_arg,
            wal_coin_arg,
        } = self;
        SitePtb {
            pt_builder,
            move_call_counter,
            bytes_estimate,
            site_argument,
            package,
            module,
            walrus_package,
            system_obj_arg,
            wal_coin_arg,
        }
    }

    fn extend_blob(&mut self, blob_ref: ObjectRef, epochs: u32) -> SitePtbBuilderResult<()> {
        self.limits_check_and_update(1, PTB_EXTEND_OPERATION_SIZE)?;
        let blob_obj_arg = self.pt_builder.obj(ObjectArg::ImmOrOwnedObject(blob_ref))?;
        let epochs_move_arg = self.pt_builder.pure(epochs)?;
        // Call walrus::system::extend_blob directly using the walrus package,
        // since add_programmable_move_call uses the sites package.
        self.pt_builder.programmable_move_call(
            self.walrus_package,
            Identifier::new(contracts::walrus::extend_blob.module)
                .expect("module name is a valid identifier"),
            contracts::walrus::extend_blob.identifier(),
            vec![],
            vec![
                self.system_obj_arg
                    .ok_or(anyhow::anyhow!("walrus System object not initialized"))?,
                blob_obj_arg,
                epochs_move_arg,
                self.wal_coin_arg
                    .ok_or(anyhow::anyhow!("WAL coin not initialized"))?,
            ],
        );
        Ok(())
    }

    fn create_wal_coin(&mut self, coins: Vec<Coin>) -> SitePtbBuilderResult<Argument> {
        // Add the first coin to the PTB
        // Note: Extreme edge case: If a user has A LOT of dust coins only and we end up
        // selecting more than 1000 coins, we will hit a transaction-limit.
        let mut coin_args: Vec<Argument> = coins
            .iter()
            .map(|coin| {
                self.pt_builder
                    .obj(ObjectArg::ImmOrOwnedObject(coin.object_ref()))
            })
            .collect::<anyhow::Result<Vec<_>, _>>()?;
        let wal_coin_arg = coin_args.remove(0);
        Ok(if !coin_args.is_empty() {
            self.increment_move_call_counter()?;
            self.pt_builder
                .command(Command::MergeCoins(wal_coin_arg, coin_args))
        } else {
            wal_coin_arg
        })
    }

    fn extract_system_arg(&mut self, system_obj: Object) -> anyhow::Result<Argument> {
        let Owner::Shared {
            initial_shared_version,
        } = system_obj.owner
        else {
            anyhow::bail!("Expect walrus System object to be shared");
        };
        self.pt_builder.obj(ObjectArg::SharedObject {
            id: system_obj.id(),
            initial_shared_version,
            mutability: SharedObjectMutability::Mutable,
        })
    }

    /// Does not update the measurements. Only checks.
    fn limits_check(&self, calls: u16, size: usize) -> Result<(), SitePtbBuilderError> {
        self.move_call_check(calls)?;
        self.ptb_size_check(size)
    }

    fn limits_check_and_update(
        &mut self,
        calls: u16,
        size: usize,
    ) -> Result<(), SitePtbBuilderError> {
        self.limits_check(calls, size)?;
        self.move_call_counter += calls;
        self.bytes_estimate += size;
        Ok(())
    }

    fn ptb_size_check(&self, size: usize) -> SitePtbBuilderResult<()> {
        if self.bytes_estimate + size > PTB_MAX_BYTES {
            return Err(SitePtbBuilderError::PtbSizeEstimationExceeded(
                self.bytes_estimate + size,
            ));
        }
        Ok(())
    }

    fn ptb_size_check_and_update(&mut self, size: usize) -> SitePtbBuilderResult<()> {
        self.ptb_size_check(size)?;
        self.bytes_estimate += size;
        Ok(())
    }

    fn move_call_check(&self, move_calls_needed: u16) -> Result<(), SitePtbBuilderError> {
        if move_calls_needed + self.move_call_counter > MAX_MOVE_CALLS {
            return Err(SitePtbBuilderError::TooManyMoveCalls(MAX_MOVE_CALLS));
        }
        Ok(())
    }

    fn increment_move_call_counter(&mut self) -> SitePtbBuilderResult<()> {
        self.move_call_check(1)?;
        self.move_call_counter += 1;
        Ok(())
    }
}

impl<const MAX_MOVE_CALLS: u16> SitePtb<Argument, MAX_MOVE_CALLS> {
    pub fn add_resource_operations<'a>(
        &mut self,
        calls: &mut std::iter::Peekable<impl Iterator<Item = &'a SiteOps<'a>>>,
    ) -> SitePtbBuilderResult<()> {
        while let Some(call) = calls.peek() {
            match call {
                SiteOps::Deleted(resource) => self.remove_resource_if_exists(resource)?,
                SiteOps::Created(resource) => self.add_resource(resource)?,
                SiteOps::RemovedRoutes => self.remove_routes()?,
                SiteOps::RemovedRedirects => self.remove_redirects()?,
                SiteOps::BurnedSite => self.burn()?,
                SiteOps::Unchanged(_) => (),
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
        self.ptb_size_check_and_update(resource.info.path.len() + PTB_STRING_PURE_ARG_OVERHEAD)?;
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
        self.ptb_size_check_and_update(Self::new_resource_ptb_size_estimation(resource))?;
        self.move_call_check(move_calls_needed)?;

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

    /// Adds the move calls to create a resource.
    ///
    /// Returns the [`Argument`] for the newly-created resource.
    fn create_resource(&mut self, resource: &Resource) -> SitePtbBuilderResult<Argument> {
        // Track estimated serialized size: path + blob_id (32 bytes) + blob_hash + overhead.
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

    fn new_resource_ptb_size_estimation(resource: &Resource) -> usize {
        let headers_size = resource.info.headers.iter().fold(0_usize, |size, h| {
            size + h.0.len() + h.1.len() + 2 * PTB_STRING_PURE_ARG_OVERHEAD
        });
        let range_size = match resource.info.range {
            None => PTB_RANGE_NONE_SIZE,
            Some(_) => PTB_RANGE_SOME_SIZE,
        };
        headers_size
            + range_size
            + resource.info.path.len()
            + PTB_STRING_PURE_ARG_OVERHEAD
            + 2 * PTB_U256_PURE_ARG_SIZE
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
        // Track estimated serialized size: key + value + BCS overhead.
        self.ptb_size_check_and_update(key.len() + value.len() + 2 * PTB_STRING_PURE_ARG_OVERHEAD)?;
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
        self.move_call_check(2)?; // remove + create routes
        self.remove_routes()?;
        self.create_routes()?;
        Ok(())
    }

    /// Adds the move calls to remove the routes object.
    // TODO: Remove pub and move RouteOp logic from `manager.rs`?
    pub fn remove_routes(&mut self) -> SitePtbBuilderResult<()> {
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

    // Redirects

    /// Adds the move calls to create an empty redirects DF on the site.
    fn create_redirects(&mut self) -> SitePtbBuilderResult<()> {
        self.add_programmable_move_call(
            contracts::site::create_redirects.identifier(),
            vec![],
            vec![self.site_argument],
        )?;
        Ok(())
    }

    /// Adds the move calls to remove the redirects object.
    pub fn remove_redirects(&mut self) -> SitePtbBuilderResult<()> {
        self.add_programmable_move_call(
            contracts::site::take_redirects_if_exist.identifier(),
            vec![],
            vec![self.site_argument],
        )?;
        Ok(())
    }

    /// Adds the move calls to remove and create new redirects.
    pub fn replace_redirects(&mut self) -> SitePtbBuilderResult<()> {
        self.move_call_check(2)?; // remove + create redirects
        self.remove_redirects()?;
        self.create_redirects()?;
        Ok(())
    }

    /// Adds the move calls to add a redirect to the redirects object.
    fn add_redirect(&mut self, path: &str, redirect: &Redirect) -> SitePtbBuilderResult<()> {
        tracing::debug!(path=%path, location=%redirect.location, status=%redirect.status_code, "new Move call: adding redirect");
        self.ptb_size_check_and_update(
            path.len()
                + redirect.location.len()
                + 2 * PTB_STRING_PURE_ARG_OVERHEAD
                + PTB_U16_PURE_ARG_SIZE,
        )?;
        let path_input = self.pt_builder.input(pure_call_arg(&path.to_owned())?)?;
        let location_input = self.pt_builder.input(pure_call_arg(&redirect.location)?)?;
        let status_code_input = self
            .pt_builder
            .input(pure_call_arg(&redirect.status_code)?)?;
        self.add_programmable_move_call(
            contracts::site::insert_redirect.identifier(),
            vec![],
            vec![
                self.site_argument,
                path_input,
                location_input,
                status_code_input,
            ],
        )?;
        Ok(())
    }

    /// Adds move calls to insert all redirect entries.
    pub fn add_redirect_operations(
        &mut self,
        new_redirects_iter: &mut std::iter::Peekable<
            std::collections::btree_map::Iter<String, Redirect>,
        >,
    ) -> SitePtbBuilderResult<()> {
        while let Some((path, redirect)) = new_redirects_iter.peek() {
            self.add_redirect(path, redirect)?;
            new_redirects_iter.next();
        }
        Ok(())
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
    fn burn(&mut self) -> SitePtbBuilderResult<()> {
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

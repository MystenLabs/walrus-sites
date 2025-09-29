// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use serde::Serialize;
use sui_types::{
    base_types::{ObjectID, SuiAddress},
    programmable_transaction_builder::ProgrammableTransactionBuilder,
    transaction::{Argument, CallArg, ProgrammableTransaction},
    Identifier,
    TypeTag,
};

use super::SitePtbBuilderError;

pub const PTB_MAX_MOVE_CALLS: u16 = 1024;

/// A simple wrapper over ProgrammableTransactionBuilder which counts move-calls in Transaction in
/// order to error explicitly if exceeded. Used by SitePtb.
#[derive(Default)]
pub struct CountedPtbBuilder<const MAX_MOVE_CALLS: u16 = PTB_MAX_MOVE_CALLS> {
    pt_builder: ProgrammableTransactionBuilder,
    move_call_counter: u16,
}

impl<const MAX_MOVE_CALLS: u16> CountedPtbBuilder<MAX_MOVE_CALLS> {
    pub fn finish(self) -> ProgrammableTransaction {
        self.pt_builder.finish()
    }

    pub fn pure<T: Serialize>(&mut self, value: T) -> anyhow::Result<Argument> {
        self.pt_builder.pure(value)
    }

    pub fn input(&mut self, call_arg: CallArg) -> anyhow::Result<Argument> {
        self.pt_builder.input(call_arg)
    }

    pub fn transfer_arg(
        &mut self,
        recipient: SuiAddress,
        arg: Argument,
    ) -> Result<(), SitePtbBuilderError> {
        self.increment_counter()?;
        self.pt_builder.transfer_arg(recipient, arg);
        Ok(())
    }

    pub fn programmable_move_call(
        &mut self,
        package: ObjectID,
        module: Identifier,
        function: Identifier,
        type_arguments: Vec<TypeTag>,
        arguments: Vec<Argument>,
    ) -> Result<Argument, SitePtbBuilderError> {
        self.increment_counter()?;
        Ok(self.pt_builder.programmable_move_call(
            package,
            module,
            function,
            type_arguments,
            arguments,
        ))
    }

    pub fn with_max_move_calls<const NEW_MAX: u16>(self) -> CountedPtbBuilder<NEW_MAX> {
        // TODO: const-assert NEW_MAX < 1024

        let Self {
            pt_builder,
            move_call_counter,
        } = self;
        CountedPtbBuilder::<{ NEW_MAX }> {
            pt_builder,
            move_call_counter,
        }
    }

    pub fn count(&self) -> u16 {
        self.move_call_counter
    }

    fn increment_counter(&mut self) -> Result<(), SitePtbBuilderError> {
        if self.move_call_counter > MAX_MOVE_CALLS {
            return Err(SitePtbBuilderError::TooManyMoveCalls(MAX_MOVE_CALLS));
        }
        self.move_call_counter += 1;
        Ok(())
    }
}

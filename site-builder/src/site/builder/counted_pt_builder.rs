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

#[derive(Default)]
pub struct CountedPTBuilder {
    pt_builder: ProgrammableTransactionBuilder,
    move_call_counter: u16,
}

impl CountedPTBuilder {
    // TODO: Maybe we can pass this as {const N} in order for the user of the struct to be able to
    // decide how many calls they want to include before erroring.
    // TODO: There are probably more limits to look out for (eg. max inputs).
    const MAX_MOVE_CALLS: u16 = 1024;

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

    fn increment_counter(&mut self) -> Result<(), SitePtbBuilderError> {
        if self.move_call_counter >= Self::MAX_MOVE_CALLS {
            return Err(SitePtbBuilderError::TooManyMoveCalls(Self::MAX_MOVE_CALLS));
        }
        self.move_call_counter += 1;
        Ok(())
    }
}

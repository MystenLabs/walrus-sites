use anyhow::Result;
use sui_types::{
    base_types::{ObjectID, SuiAddress},
    programmable_transaction_builder::ProgrammableTransactionBuilder,
    transaction::{Argument, ProgrammableTransaction},
    Identifier, TypeTag,
};

#[macro_use]
mod macros {

    macro_rules! move_call_inner {
        ($self:ident, $move_function:expr, $($args:ident), *) => {
            Ok($self.add_programmable_move_call(
                Identifier::new($move_function)?,
                vec![],
                vec![$($args), *],
            ))
        };
    }

    // A macro to generate a function the inner move function
    // The arguments to the function are specified in the macro call
    macro_rules! move_call {
        ($name:ident, $($args:ident), *) => {
            pub fn $name(&mut self, $($args: Argument), *) -> Result<Argument> {
                move_call_inner!(self, stringify!($name), $($args), *)
            }
        };
    }
}

pub struct CallBuilder {
    pub pt_builder: ProgrammableTransactionBuilder,
    package: ObjectID,
    module: Identifier,
}

impl CallBuilder {
    pub fn new(package: ObjectID, module: Identifier) -> Self {
        CallBuilder {
            pt_builder: ProgrammableTransactionBuilder::new(),
            package,
            module,
        }
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

    pub fn finish(self) -> ProgrammableTransaction {
        self.pt_builder.finish()
    }

    pub fn transfer_arg(&mut self, recipient: SuiAddress, arg: Argument) {
        self.pt_builder.transfer_arg(recipient, arg);
    }

    // Calls
    move_call!(new_site, name, clock);
    move_call!(
        new_page,
        name,
        content_type,
        content_encoding,
        contents,
        clock
    );
    move_call!(add_page, site, page);
    // move_call!(remove_page, site, name);
    move_call!(add_piece, page, piece, clock);
}

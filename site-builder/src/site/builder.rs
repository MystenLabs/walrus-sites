use anyhow::Result;
use serde::Serialize;
use sui_types::{
    base_types::{ObjectID, SuiAddress},
    programmable_transaction_builder::ProgrammableTransactionBuilder,
    transaction::{Argument, CallArg, ProgrammableTransaction},
    Identifier,
    TypeTag,
};

pub struct BlocksitePtb<T = ()> {
    pt_builder: ProgrammableTransactionBuilder,
    site_argument: T,
    package: ObjectID,
    module: Identifier,
}

/// A PTB to update part of a blocksite
/// It is composed of a series of [BlocksiteCall]s, which all have the
/// blocksite object id as first argument.
impl BlocksitePtb {
    pub fn new(package: ObjectID, module: Identifier) -> Result<Self> {
        let pt_builder = ProgrammableTransactionBuilder::new();
        Ok(BlocksitePtb {
            pt_builder,
            site_argument: (),
            package,
            module,
        })
    }

    pub fn with_call_arg(mut self, site_arg: &CallArg) -> Result<BlocksitePtb<Argument>> {
        let site_argument = self.pt_builder.input(site_arg.clone())?;
        Ok(BlocksitePtb {
            pt_builder: self.pt_builder,
            site_argument,
            package: self.package,
            module: self.module,
        })
    }

    pub fn with_arg(self, site_arg: Argument) -> Result<BlocksitePtb<Argument>> {
        Ok(BlocksitePtb {
            pt_builder: self.pt_builder,
            site_argument: site_arg,
            package: self.package,
            module: self.module,
        })
    }
}

impl<T> BlocksitePtb<T> {
    /// Move call to create a new blocksite
    pub fn create_site(&mut self, site_name: &str) -> Result<Argument> {
        let name_arg = self.pt_builder.input(pure_call_arg(&site_name)?)?;
        let clock_arg = self.pt_builder.input(CallArg::CLOCK_IMM)?;
        Ok(self.add_programmable_move_call(
            Identifier::new("new_site")?,
            vec![],
            vec![name_arg, clock_arg],
        ))
    }

    /// Transfer argument to address
    pub fn transfer_arg(&mut self, recipient: SuiAddress, arg: Argument) {
        self.pt_builder.transfer_arg(recipient, arg);
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
}

impl BlocksitePtb<Argument> {
    pub fn add_calls(&mut self, calls: Vec<BlocksiteCall>) -> Result<()> {
        for call in calls {
            self.add_call(call)?;
        }
        Ok(())
    }

    pub fn add_call(&mut self, mut call: BlocksiteCall) -> Result<()> {
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
            call.function = "add_resource".to_owned();
        }
        args.insert(0, self.site_argument);
        self.add_programmable_move_call(Identifier::new(call.function)?, vec![], args);
        Ok(())
    }
}

// Testing out
#[derive(Debug)]
pub struct BlocksiteCall {
    function: String,
    args: Vec<CallArg>,
}

impl BlocksiteCall {
    /// This call results into two transactions in a PTB, one to
    /// create the resource, and one to add it to the site
    pub fn new_resource_and_add(
        resource_name: &str,
        content_type: &str,
        content_encoding: &str,
        contents: &[u8],
    ) -> Result<BlocksiteCall> {
        tracing::info!("New Move call: Creating {}", resource_name);
        Ok(BlocksiteCall {
            function: "new_resource_and_add".to_owned(),
            args: vec![
                pure_call_arg(&resource_name)?,
                pure_call_arg(&content_type)?,
                pure_call_arg(&content_encoding)?,
                pure_call_arg(&contents)?,
                CallArg::CLOCK_IMM,
            ],
        })
    }

    pub fn add_piece_to_existing(resource_name: &str, piece: &[u8]) -> Result<BlocksiteCall> {
        tracing::info!("New Move call: Adding piece to {}", resource_name);
        Ok(BlocksiteCall {
            function: "add_piece_to_existing".to_owned(),
            args: vec![
                pure_call_arg(&resource_name)?,
                pure_call_arg(&piece)?,
                CallArg::CLOCK_IMM,
            ],
        })
    }

    pub fn move_resource(old_name: &str, new_name: &str) -> Result<BlocksiteCall> {
        tracing::info!("New Move call: Moving {} to {}", old_name, new_name);
        Ok(BlocksiteCall {
            function: "move_resource".to_owned(),
            args: vec![pure_call_arg(&old_name)?, pure_call_arg(&new_name)?],
        })
    }

    pub fn remove_resource_if_exists(resource_name: &str) -> Result<BlocksiteCall> {
        tracing::info!("New Move call: Removing {}", resource_name);
        Ok(BlocksiteCall {
            function: "remove_resource_if_exists".to_owned(),
            args: vec![pure_call_arg(&resource_name)?],
        })
    }
}

pub fn pure_call_arg<T: Serialize>(arg: &T) -> Result<CallArg> {
    Ok(CallArg::Pure(bcs::to_bytes(arg)?))
}

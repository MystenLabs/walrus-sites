// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Walrus Sites contract bindings. Provides an interface for looking up contract function, modules,
//! and type names.

use core::fmt;

use anyhow::{anyhow, Context, Result};
use move_core_types::{identifier::Identifier, language_storage::StructTag as MoveStructTag};
use serde::de::DeserializeOwned;
use sui_sdk::{
    rpc_types::{SuiData, SuiObjectData, SuiObjectDataOptions, SuiObjectResponse},
    types::base_types::ObjectID,
    SuiClient,
};
use sui_types::TypeTag;
use tracing::instrument;

/// A trait for types that correspond to a contract type.
///
/// Implementors of this trait are convertible from [SuiObjectData]s and can
/// identify their associated contract type.
pub trait AssociatedContractStruct: DeserializeOwned {
    /// [`StructTag`] corresponding to the Move struct associated type.
    const CONTRACT_STRUCT: StructTag<'static>;

    /// Converts a [`SuiObjectData`] to [`Self`].
    #[instrument(err, skip_all)]
    fn try_from_object_data(sui_object_data: &SuiObjectData) -> Result<Self> {
        tracing::debug!(
            "converting move object to rust struct {:?}",
            Self::CONTRACT_STRUCT,
        );
        let raw = sui_object_data
            .bcs
            .as_ref()
            .ok_or(anyhow!("no bcs representation received"))?;
        let raw = raw
            .try_as_move()
            .ok_or(anyhow!("the data requested is not a move object"))?;
        assert!(
            raw.type_.name.as_str() == Self::CONTRACT_STRUCT.name
                && raw.type_.module.as_str() == Self::CONTRACT_STRUCT.module,
            "the returned object does not match the expected type"
        );
        Ok(bcs::from_bytes(&raw.bcs_bytes)?)
    }
}

/// Tag identifying contract functions based on their name and module.
#[derive(Debug)]
pub struct FunctionTag<'a> {
    /// Move function name.
    pub name: &'a str,
    /// Move module of the function.
    pub module: &'a str,
    /// Type parameters of the function.
    pub type_params: Vec<TypeTag>,
    /// Number of Sui objects that are outputs of the function.
    pub n_object_outputs: u16,
}

impl<'a> FunctionTag<'a> {
    /// Return a new [FunctionTag] with the provided type parameters.
    #[allow(dead_code)]
    pub fn with_type_params(&self, type_params: &[TypeTag]) -> Self {
        Self {
            type_params: type_params.to_vec(),
            ..*self
        }
    }

    pub fn identifier(&self) -> Identifier {
        Identifier::new(self.name).expect("function name is a valid identifier")
    }
}

/// Tag identifying contract structs based on their name and module.
#[derive(Debug, PartialEq, Eq)]
pub struct StructTag<'a> {
    /// Move struct name.
    pub name: &'a str,
    /// Move module of the struct.
    pub module: &'a str,
}

#[allow(dead_code)]
impl<'a> StructTag<'a> {
    /// Returns a Move StructTag for the identified struct, within the published contract module.
    pub fn to_move_struct_tag(
        &self,
        package: ObjectID,
        type_params: &[TypeTag],
    ) -> Result<MoveStructTag> {
        Ok(MoveStructTag {
            address: package.into(),
            module: Identifier::new(self.module).with_context(|| {
                format!("Struct module is not a valid identifier: {}", self.module)
            })?,
            name: Identifier::new(self.name).with_context(|| {
                format!("Struct name is not a valid identifier: {}", self.module)
            })?,
            type_params: type_params.into(),
        })
    }
}

impl<'a> From<&'a MoveStructTag> for StructTag<'a> {
    fn from(value: &'a MoveStructTag) -> Self {
        Self {
            name: value.name.as_str(),
            module: value.module.as_str(),
        }
    }
}

impl<'a> fmt::Display for StructTag<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}::{}", self.module, self.name)
    }
}

pub(crate) async fn get_sui_object<U>(sui_client: &SuiClient, object_id: ObjectID) -> Result<U>
where
    U: AssociatedContractStruct,
{
    get_sui_object_from_object_response(
        &sui_client
            .read_api()
            .get_object_with_options(
                object_id,
                SuiObjectDataOptions::new().with_bcs().with_type(),
            )
            .await?,
    )
}

pub(crate) fn get_sui_object_from_object_response<U>(
    object_response: &SuiObjectResponse,
) -> Result<U>
where
    U: AssociatedContractStruct,
{
    U::try_from_object_data(
        object_response
            .data
            .as_ref()
            .ok_or_else(|| anyhow!("response does not contain object data"))?,
    )
    .map_err(|_e| {
        anyhow!(
            "could not convert object to expected type {}",
            U::CONTRACT_STRUCT
        )
    })
}

macro_rules! contract_ident {
    (struct $modname:ident::$itemname:ident) => {
        #[allow(non_upper_case_globals)]
        #[doc=stringify!([StructTag] for the Move struct $modname::$itemname)]
        pub const $itemname: StructTag = StructTag {
            module: stringify!($modname),
            name: stringify!($itemname),
        };
    };
    (fn $modname:ident::$itemname:ident) => {
        contract_ident!(fn $modname::$itemname, 0);
    };
    (fn $modname:ident::$itemname:ident, $n_out:expr) => {
        #[allow(non_upper_case_globals)]
        #[doc=stringify!([FunctionTag] for the Move function $modname::$itemname)]
        pub const $itemname: FunctionTag = FunctionTag {
            module: stringify!($modname),
            name: stringify!($itemname),
            type_params: vec![],
            n_object_outputs: $n_out,
        };
    };
}

pub mod site {
    use super::*;

    contract_ident!(fn site::new_site, 1);
    // Resource functions
    contract_ident!(struct site::Resource);
    contract_ident!(struct site::ResourcePath);
    contract_ident!(fn site::remove_resource_if_exists, 1);
    contract_ident!(fn site::new_resource);
    contract_ident!(fn site::add_resource);
    contract_ident!(fn site::add_header);
    // Routes functions
    contract_ident!(struct site::Routes);
    contract_ident!(fn site::create_routes, 1);
    contract_ident!(fn site::insert_route);
    contract_ident!(fn site::remove_all_routes_if_exist, 1);
    // Range functions
    //contract_ident!(struct site::Range);
    contract_ident!(fn site::new_range_option, 1);
}

pub mod dynamic_field {
    use super::*;

    contract_ident!(struct dynamic_field::Field);
}

// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Walrus Sites contract bindings. Provides an interface for looking up contract function, modules,
//! and type names.

use core::fmt;
use std::collections::BTreeMap;

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
            contract_struct = ?Self::CONTRACT_STRUCT,
            object_id = ?sui_object_data.object_id,
            "converting move object to rust struct",
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
#[allow(unused)]
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

impl FunctionTag<'_> {
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

pub(crate) type TypeOriginMap = BTreeMap<(String, String), ObjectID>;

/// Tag identifying contract structs based on their name and module.
#[derive(Debug, PartialEq, Eq)]
pub struct StructTag<'a> {
    /// Move struct name.
    pub name: &'a str,
    /// Move module of the struct.
    pub module: &'a str,
}

impl StructTag<'_> {
    /// Returns a [MoveStructTag] for the identified struct with the given package ID.
    ///
    /// Use [`Self::to_move_struct_tag_with_type_map`] if the type origin map is available.
    pub(crate) fn to_move_struct_tag_with_package(
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

    /// Converts a [StructTag] to a [MoveStructTag] using the matching package ID from the given
    /// type origin map.
    pub(crate) fn to_move_struct_tag_with_type_map(
        &self,
        type_origin_map: &TypeOriginMap,
        type_params: &[TypeTag],
    ) -> Result<MoveStructTag> {
        let package_id = type_origin_map
            .get(&(self.module.to_string(), self.name.to_string()))
            .ok_or(anyhow::anyhow!("type origin not found"))?;
        self.to_move_struct_tag_with_package(*package_id, type_params)
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

impl fmt::Display for StructTag<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}::{}", self.module, self.name)
    }
}

pub async fn get_sui_object<U>(sui_client: &SuiClient, object_id: ObjectID) -> Result<U>
where
    U: AssociatedContractStruct,
{
    let start = std::time::Instant::now();
    get_sui_object_from_object_response(
        &sui_client
            .read_api()
            .get_object_with_options(
                object_id,
                SuiObjectDataOptions::new().with_bcs().with_type(),
            )
            .await?,
    )
    .inspect(|_| {
        tracing::debug!(
            %object_id,
            elapsed = ?start.elapsed(),
            "got sui object",
        )
    })
}

pub fn get_sui_object_from_object_response<U>(object_response: &SuiObjectResponse) -> Result<U>
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

    contract_ident!(struct site::Site);
    contract_ident!(fn site::new_site, 1);
    contract_ident!(fn site::burn);
    contract_ident!(fn site::update_metadata);
    contract_ident!(fn site::update_name);
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
    contract_ident!(fn site::new_range_option, 1);
}

pub mod walrus {
    use super::*;

    contract_ident!(struct blob::Blob);
}

pub mod dynamic_field {
    use super::*;

    contract_ident!(struct dynamic_field::Field);
}

pub mod suins {
    use super::*;

    contract_ident!(struct name_record::NameRecord);
    contract_ident!(struct domain::Domain);
}

pub mod staking {
    use super::*;

    contract_ident!(struct staking::Staking);
}
pub mod staking_inner {
    use super::*;

    contract_ident!(struct staking_inner::StakingInnerV1);
}

// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Walrus Sites contract bindings. Provides an interface for looking up contract function, modules,
//! and type names.

use anyhow::Context;
use sui_sdk::{
    rpc_types::{SuiObjectDataOptions, SuiObjectResponse},
    SuiClient,
};
pub use walrus_sdk::sui::contracts::{
    AssociatedContractStruct,
    FunctionTag,
    StructTag,
    TypeOriginMap,
};
use walrus_sdk::ObjectID;
pub use walrus_sui::contract_ident;

pub async fn get_sui_object<U>(sui_client: &SuiClient, object_id: ObjectID) -> anyhow::Result<U>
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

pub fn get_sui_object_from_object_response<U>(
    object_response: &SuiObjectResponse,
) -> anyhow::Result<U>
where
    U: AssociatedContractStruct,
{
    U::try_from_object_data(
        object_response
            .data
            .as_ref()
            .context("response does not contain object data")?,
    )
    .with_context(|| {
        format!(
            "could not convert object to expected type {}",
            U::CONTRACT_STRUCT
        )
    })
}

pub mod site {
    walrus_sui::contract_ident!(struct site::Site);
    walrus_sui::contract_ident!(fn site::new_site);
    walrus_sui::contract_ident!(fn site::burn);
    walrus_sui::contract_ident!(fn site::update_metadata);
    walrus_sui::contract_ident!(fn site::update_name);
    // Resource functions
    walrus_sui::contract_ident!(struct site::Resource);
    walrus_sui::contract_ident!(struct site::ResourcePath);
    walrus_sui::contract_ident!(fn site::remove_resource_if_exists);
    walrus_sui::contract_ident!(fn site::new_resource);
    walrus_sui::contract_ident!(fn site::add_resource);
    walrus_sui::contract_ident!(fn site::add_header);
    // Routes functions
    walrus_sui::contract_ident!(struct site::Routes);
    walrus_sui::contract_ident!(fn site::create_routes);
    walrus_sui::contract_ident!(fn site::insert_route);
    walrus_sui::contract_ident!(fn site::remove_all_routes_if_exist);
    // Range functions
    walrus_sui::contract_ident!(fn site::new_range_option);
}

pub mod walrus {
    walrus_sui::contract_ident!(struct blob::Blob);
    walrus_sui::contract_ident!(fn system::extend_blob);
}

pub mod dynamic_field {
    walrus_sui::contract_ident!(struct dynamic_field::Field);
}

pub mod suins {
    walrus_sui::contract_ident!(struct name_record::NameRecord);
    walrus_sui::contract_ident!(struct domain::Domain);
}

pub mod staking {
    walrus_sui::contract_ident!(struct staking::Staking);
}

pub mod staking_inner {
    walrus_sui::contract_ident!(struct staking_inner::StakingInnerV1);
}

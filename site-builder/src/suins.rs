// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Utilities to resolve SuiNS addresses.

use anyhow::{bail, Result};
use sui_types::{base_types::ObjectID, TypeTag};

use crate::{
    retry_client::RetriableSuiClient,
    site::contracts::{self, TypeOriginMap},
    types::{Domain, NameRecord},
    util::type_origin_map_for_package,
};

/// A static config containing the SuiNS addresses for testnet and mainnet.
mod suins_config {
    use anyhow::{bail, Result};
    use sui_types::base_types::ObjectID;

    /// Returns the SuiNS registry object ID and the SuiNS package object ID for mainnet.
    pub fn mainnet() -> (ObjectID, ObjectID) {
        (
            ObjectID::from_hex_literal(
                "0xe64cd9db9f829c6cc405d9790bd71567ae07259855f4fba6f02c84f52298c106",
            )
            .expect("is a valid object ID"),
            ObjectID::from_hex_literal(
                "0xd22b24490e0bae52676651b4f56660a5ff8022a2576e0089f79b3c88d44e08f0",
            )
            .expect("is a valid object ID"),
        )
    }

    /// Returns the SuiNS registry object ID and the SuiNS package object ID for testnet.
    pub fn testnet() -> (ObjectID, ObjectID) {
        (
            ObjectID::from_hex_literal(
                "0xb120c0d55432630fce61f7854795a3463deb6e3b443cc4ae72e1282073ff56e4",
            )
            .expect("is a valid object ID"),
            ObjectID::from_hex_literal(
                "0x22fa05f21b1ad71442491220bb9338f7b7095fe35000ef88d5400d28523bdd93",
            )
            .expect("is a valid object ID"),
        )
    }

    pub fn try_for_context(context: &str) -> Result<(ObjectID, ObjectID)> {
        match context {
            "testnet" => Ok(testnet()),
            "mainnet" => Ok(mainnet()),
            _ => bail!(
                "invalid context: {context}, the only two contexts supporting SuiNS \
                resolution are 'testnet' or 'mainnet'"
            ),
        }
    }
}

/// A client to resolve SuiNS.
pub(crate) struct SuiNsClient {
    /// The client used for the resolution.
    client: RetriableSuiClient,
    /// The object ID of the SuiNS registry.
    suins_object_id: ObjectID,
    /// The type map for the SuiNS package.
    type_map: TypeOriginMap,
}

impl SuiNsClient {
    /// Creates a new SuiNS client.
    pub(crate) async fn new(
        client: RetriableSuiClient,
        suins_object_id: ObjectID,
        package_id: ObjectID,
    ) -> Result<Self> {
        let type_map = type_origin_map_for_package(&client, package_id).await?;
        Ok(Self {
            client,
            suins_object_id,
            type_map,
        })
    }

    /// Creates a new SuiNS client from the given context.
    pub(crate) async fn from_context(client: RetriableSuiClient, context: &str) -> Result<Self> {
        let (suins_object, package_id) = suins_config::try_for_context(context)?;
        Self::new(client, suins_object, package_id).await
    }

    /// Resolves the SuiNS name record for the given name.
    pub(crate) async fn resolve_name_record(&self, name: &str) -> Result<NameRecord> {
        let domain_type = self.get_domain_type()?;
        let Some(normalized_name) = Domain::from_name(name) else {
            bail!("invalid SuiNS name: {name}, must be of the form <name>.sui");
        };

        tracing::debug!(
            ?normalized_name,
            ?domain_type,
            "resolving SuiNS name record"
        );

        let dynamic_field = self
            .client
            .get_dynamic_field::<Domain, NameRecord>(
                self.suins_object_id,
                domain_type,
                normalized_name,
            )
            .await?;

        Ok(dynamic_field)
    }

    /// Gets the type tag for the domain.
    pub(crate) fn get_domain_type(&self) -> Result<TypeTag> {
        Ok(contracts::suins::Domain
            .to_move_struct_tag_with_type_map(&self.type_map, &[])?
            .into())
    }
}

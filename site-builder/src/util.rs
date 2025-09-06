// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0
use std::{
    path::{Path, PathBuf},
    str,
};

use anyhow::{anyhow, bail, Context, Result};
use futures::Future;
use serde::{Deserialize, Deserializer};
use sui_keys::keystore::AccountKeystore;
use sui_sdk::{
    rpc_types::{
        Page,
        SuiObjectDataOptions,
        SuiRawData,
        SuiTransactionBlockEffects,
        SuiTransactionBlockEffectsAPI,
        SuiTransactionBlockResponse,
    },
    wallet_context::WalletContext,
};
use sui_types::{
    base_types::{ObjectID, ObjectRef, SuiAddress},
    transaction::{ProgrammableTransaction, TransactionData},
    TypeTag,
};

use crate::{
    display,
    retry_client::RetriableSuiClient,
    site::{config::WSResources, contracts::TypeOriginMap},
    types::{Staking, StakingInnerV1, StakingObjectForDeserialization},
};

pub async fn sign_and_send_ptb(
    active_address: SuiAddress,
    wallet: &WalletContext,
    retry_client: &RetriableSuiClient,
    programmable_transaction: ProgrammableTransaction,
    gas_coin: ObjectRef,
    gas_budget: u64,
) -> Result<SuiTransactionBlockResponse> {
    let gas_price = wallet.get_reference_gas_price().await?;
    let transaction = TransactionData::new_programmable(
        active_address,
        vec![gas_coin],
        programmable_transaction,
        gas_budget,
        gas_price,
    );
    let transaction = wallet.sign_transaction(&transaction);
    retry_client.execute_transaction(transaction).await
}

pub async fn handle_pagination<F, T, C, Fut>(
    closure: F,
) -> Result<impl Iterator<Item = T>, sui_sdk::error::Error>
where
    F: FnMut(Option<C>) -> Fut,
    T: 'static,
    Fut: Future<Output = Result<Page<T, C>, sui_sdk::error::Error>>,
{
    handle_pagination_with_cursor(closure, None).await
}

pub(crate) async fn handle_pagination_with_cursor<F, T, C, Fut>(
    mut closure: F,
    mut cursor: Option<C>,
) -> Result<impl Iterator<Item = T>, sui_sdk::error::Error>
where
    F: FnMut(Option<C>) -> Fut,
    T: 'static,
    Fut: Future<Output = Result<Page<T, C>, sui_sdk::error::Error>>,
{
    let mut cont = true;
    let mut iterators = vec![];
    while cont {
        let page = closure(cursor).await?;
        cont = page.has_next_page;
        cursor = page.next_cursor;
        iterators.push(page.data.into_iter());
    }
    Ok(iterators.into_iter().flatten())
}

pub fn bytes_to_base36(source: &[u8]) -> Result<String> {
    const BASE36: &[u8] = "0123456789abcdefghijklmnopqrstuvwxyz".as_bytes();
    let base = BASE36.len();
    let size = source.len() * 2;
    let mut encoding = vec![0; size];
    let mut high = size - 1;
    for digit in source {
        let mut carry = *digit as usize;
        let mut it = size - 1;
        while it > high || carry != 0 {
            carry += 256 * encoding[it];
            encoding[it] = carry % base;
            carry /= base;
            it -= 1;
        }
        high = it;
    }
    let skip = encoding.iter().take_while(|v| **v == 0).count();
    let string = str::from_utf8(
        &(encoding[skip..]
            .iter()
            .map(|&c| BASE36[c])
            .collect::<Vec<_>>()),
    )
    .unwrap()
    .to_owned();
    Ok(string)
}

pub fn str_to_base36(input: &str) -> Result<String> {
    bytes_to_base36(input.as_bytes())
}

/// Convert the hex representation of an object id to base36.
pub fn id_to_base36(id: &ObjectID) -> Result<String> {
    bytes_to_base36(&id.into_bytes())
}

/// Get the object id of the site that was published in the transaction.
///
/// # Panics
///
/// Panics if the created site object ID cannot be found in the transaction effects.
/// This can happen if, for example, no object owned by the provided `address` was created
/// in the transaction, or if the transaction did not result in the expected object creation
/// structure that this function relies on.
pub fn get_site_id_from_response(
    address: SuiAddress,
    effects: &SuiTransactionBlockEffects,
) -> ObjectID {
    // Return type changed to ObjectID
    tracing::debug!(
        ?effects,
        "getting the object ID of the created Walrus site."
    );
    effects
        .created()
        .iter()
        .find(|c| {
            c.owner
                .get_owner_address()
                .map(|owner_address| owner_address == address)
                .unwrap_or(false)
        })
        .expect("could not find the object ID for the created Walrus site.")
        .reference
        .object_id
}

/// Returns the path if it is `Some` or any of the default paths if they exist (attempt in order).
pub fn path_or_defaults_if_exist(path: Option<&Path>, defaults: &[PathBuf]) -> Option<PathBuf> {
    let mut path = path.map(|p| p.to_path_buf());
    for default in defaults {
        if path.is_some() {
            break;
        }
        path = default.exists().then_some(default.clone());
    }
    path
}

/// Gets the type origin map for a given package.
pub(crate) async fn type_origin_map_for_package(
    sui_client: &RetriableSuiClient,
    package_id: ObjectID,
) -> Result<TypeOriginMap> {
    let Ok(Some(SuiRawData::Package(raw_package))) = sui_client
        .get_object_with_options(
            package_id,
            SuiObjectDataOptions::default().with_type().with_bcs(),
        )
        .await?
        .into_object()
        .map(|object| object.bcs)
    else {
        bail!("no package foundwith ID {package_id}");
    };
    Ok(raw_package
        .type_origin_table
        .into_iter()
        .map(|origin| ((origin.module_name, origin.datatype_name), origin.package))
        .collect())
}

/// Loads the wallet context from the given optional wallet config (optional path and optional
/// Sui env).
///
/// If no path is provided, tries to load the configuration first from the local folder, and
/// then from the standard Sui configuration directory.
// NB: When making changes to the logic, make sure to update the argument docs in
// `crates/walrus-service/bin/client.rs`.
pub fn load_wallet_context(
    path: Option<&Path>,
    wallet_env: Option<&str>,
    wallet_address: Option<&SuiAddress>,
) -> Result<WalletContext> {
    let mut default_paths = vec!["./client.yaml".into(), "./sui_config.yaml".into()];
    if let Some(home_dir) = home::home_dir() {
        default_paths.push(home_dir.join(".sui").join("sui_config").join("client.yaml"))
    }

    let path = path_or_defaults_if_exist(path, &default_paths)
        .ok_or(anyhow!("could not find a valid wallet config file"))?;
    tracing::info!(conf_path = %path.display(), "using Sui wallet configuration");
    let mut wallet_context: WalletContext = WalletContext::new(&path)?;

    if let Some(target_env) = wallet_env {
        if !wallet_context
            .config
            .envs
            .iter()
            .any(|env| env.alias == target_env)
        {
            return Err(anyhow!(
                "Env '{}' not found in wallet config file '{}'.",
                target_env,
                path.display()
            ));
        }
        wallet_context.config.active_env = Some(target_env.to_string());
        tracing::info!(?target_env, "set the wallet env");
    } else {
        tracing::info!(
            active_env=?wallet_context.config.active_env,
            "no wallet env provided, using the default one"
        );
    }

    if let Some(target_address) = wallet_address {
        if !wallet_context
            .config
            .keystore
            .addresses()
            .iter()
            .any(|address| address == target_address)
        {
            return Err(anyhow!(
                "Address '{}' not found in wallet config file '{}'.",
                target_address,
                path.display()
            ));
        }
        wallet_context.config.active_address = Some(*target_address);
        tracing::info!(?target_address, "set the wallet address");
    } else {
        tracing::info!(
            active_address=?wallet_context.config.active_address,
            "no wallet address provided, using the default one"
        );
    }

    Ok(wallet_context)
}

/// Persists the site_object_id and site_name to the ws-resources.json file.
///
/// > Note: This function should be called only after a successful deployment operation.
pub fn persist_site_id_and_name(
    site_object_id: ObjectID,
    site_name: Option<String>,
    initial_ws_resources_opt: Option<WSResources>,
    ws_resources_path: &Path,
) -> Result<WSResources, anyhow::Error> {
    let mut ws_resources_to_save = initial_ws_resources_opt.unwrap_or_default();

    // Update/Set the site_object_id
    if ws_resources_to_save.object_id != Some(site_object_id) {
        tracing::debug!(
            "Updating site_object_id in ws-resources.json from {:?} to: {}",
            ws_resources_to_save.object_id,
            site_object_id
        );
        ws_resources_to_save.object_id = Some(site_object_id);
    } else {
        tracing::debug!(
            "Site object ID ({}) to be persisted is already the current ID in ws-resources.json.",
            site_object_id
        );
    }

    // Update/Set the site_name
    match site_name {
        Some(ref new_site_name) if ws_resources_to_save.site_name != site_name => {
            tracing::debug!(
                "Updating site_name in ws-resources.json from {:?} to: {}",
                ws_resources_to_save.site_name,
                new_site_name
            );
            ws_resources_to_save.site_name = site_name;
        }
        Some(ref existing_site_name) => {
            tracing::debug!(
                "Site Name ({}) to be persisted is already the current Site Name in ws-resources.json.",
                existing_site_name
            );
        }
        None => {
            tracing::debug!("Persisting the Default Site Name in ws-resources.json.");
        }
    }

    // Save the updated WSResources struct
    let action_message = if ws_resources_path.exists() {
        "Updating"
    } else {
        "Creating"
    };
    display::action(format!(
        "{} ws-resources.json (Site Object ID: {}, Name: {:?}) at: {}",
        action_message,
        ws_resources_to_save
            .object_id
            .expect("ID should be set by now"), // Should be Some
        ws_resources_to_save.site_name,
        ws_resources_path.display()
    ));

    ws_resources_to_save
        .save(ws_resources_path)
        .context(format!(
            "Failed to save ws-resources.json to {}",
            ws_resources_path.display()
        ))?;

    display::done();
    Ok(ws_resources_to_save)
}

/// Fetches the staking object by its ID and the current walrus package ID.
/// Returns a `StakingObject` that includes version, package IDs, and staking parameters.
pub async fn get_staking_object(
    sui_client: &RetriableSuiClient,
    staking_object_id: ObjectID,
) -> Result<Staking> {
    let StakingObjectForDeserialization {
        id,
        version,
        package_id,
        new_package_id,
    } = sui_client
        .get_sui_object(staking_object_id)
        .await
        .context("Failed to fetch staking object data")?;

    let inner = sui_client
        .get_dynamic_field::<u64, StakingInnerV1>(staking_object_id, TypeTag::U64, version)
        .await
        .context("Failed to fetch inner staking data")?;

    Ok(Staking {
        id,
        version,
        package_id,
        new_package_id,
        inner,
    })
}

#[tracing::instrument(err, skip_all)]
pub(crate) fn deserialize_bag_or_table<'de, D>(deserializer: D) -> Result<ObjectID, D::Error>
where
    D: Deserializer<'de>,
{
    let (object_id, _length): (ObjectID, u64) = Deserialize::deserialize(deserializer)?;
    Ok(object_id)
}

// Resolution

#[cfg(test)]
mod test_util {
    use sui_types::base_types::ObjectID;

    use super::*;

    #[test]
    fn test_id_to_base36() {
        let id = ObjectID::from_hex_literal(
            "0x05fb8843a23017cbf1c907bd559a2d6191b77bc595d4c83853cca14cc784c0a8",
        )
        .unwrap();
        let converted = id_to_base36(&id).unwrap();
        assert_eq!(
            &converted,
            "5d8t4gd5q8x4xcfyctpygyr5pnk85x54o7ndeq2j4pg9l7rmw"
        );
    }
}

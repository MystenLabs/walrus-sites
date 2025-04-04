// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0
use std::{
    path::{Path, PathBuf},
    str,
};

use anyhow::{anyhow, bail, Result};
use futures::Future;
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
};

use crate::{retry_client::RetriableSuiClient, site::contracts::TypeOriginMap};

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

/// Convert the hex representation of an object id to base36.
pub fn id_to_base36(id: &ObjectID) -> Result<String> {
    const BASE36: &[u8] = "0123456789abcdefghijklmnopqrstuvwxyz".as_bytes();
    let source = id.into_bytes();
    let base = BASE36.len();
    let size = source.len() * 2;
    let mut encoding = vec![0; size];
    let mut high = size - 1;
    for digit in &source {
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

/// Get the object id of the site that was published in the transaction.
#[allow(dead_code)]
pub fn get_site_id_from_response(
    address: SuiAddress,
    effects: &SuiTransactionBlockEffects,
) -> Result<ObjectID> {
    tracing::debug!(
        ?effects,
        "getting the object ID of the created Walrus site."
    );
    Ok(effects
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
        .object_id)
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
    let mut wallet_context: WalletContext = WalletContext::new(&path, None, None)?;

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
        tracing::info!("no wallet env provided, using the default one");
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
        tracing::info!("no wallet address provided, using the default one");
    }

    Ok(wallet_context)
}

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

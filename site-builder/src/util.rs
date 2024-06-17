// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, path::PathBuf, str};

use anyhow::{anyhow, Result};
use futures::Future;
use sui_sdk::{
    rpc_types::{
        Page,
        SuiMoveStruct,
        SuiObjectResponse,
        SuiParsedData,
        SuiTransactionBlockEffects,
        SuiTransactionBlockEffectsAPI,
    },
    wallet_context::WalletContext,
    SuiClient,
};
use sui_types::{
    base_types::{ObjectID, SuiAddress},
    dynamic_field::DynamicFieldInfo,
};

pub async fn get_all_dynamic_field_info(
    client: &SuiClient,
    object_id: ObjectID,
) -> Result<Vec<DynamicFieldInfo>> {
    let iter = handle_pagination(|cursor| {
        client
            .read_api()
            .get_dynamic_fields(object_id, cursor, None)
    })
    .await?
    .collect();
    Ok(iter)
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

/// Convert the hex representation of an object id to base36
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

/// Get the object id of the site that was published in the transaction
#[allow(dead_code)]
pub fn get_site_id_from_response(
    address: SuiAddress,
    effects: &SuiTransactionBlockEffects,
) -> Result<ObjectID> {
    Ok(effects
        .created()
        .iter()
        .find(|c| c.owner == address)
        .expect("Could not find the object ID for the created Walrus site.")
        .reference
        .object_id)
}

pub(crate) fn get_struct_from_object_response(
    object_response: &SuiObjectResponse,
) -> Result<SuiMoveStruct> {
    match object_response {
        SuiObjectResponse {
            data: Some(data),
            error: None,
        } => match &data.content {
            Some(SuiParsedData::MoveObject(parsed_object)) => Ok(parsed_object.fields.clone()),
            _ => Err(anyhow!("Unexpected data in ObjectResponse: {:?}", data)),
        },
        SuiObjectResponse {
            error: Some(error), ..
        } => Err(anyhow!("Error in ObjectResponse: {:?}", error)),
        SuiObjectResponse { .. } => Err(anyhow!(
            "ObjectResponse contains data and error: {:?}",
            object_response
        )),
    }
}

pub async fn get_existing_resource_ids(
    client: &SuiClient,
    site_id: ObjectID,
) -> Result<HashMap<String, ObjectID>> {
    let info = get_all_dynamic_field_info(client, site_id).await?;
    Ok(info
        .iter()
        .filter_map(|d| get_path_from_info(d).map(|path| (path, d.object_id)))
        .collect::<HashMap<String, ObjectID>>())
}

// TODO(giac): check the type of the name.
fn get_path_from_info(info: &DynamicFieldInfo) -> Option<String> {
    info.name
        .value
        .as_object()
        .and_then(|obj| obj.get("path"))
        .and_then(|p| p.as_str())
        .map(|s| s.to_owned())
}

/// Returns the path if it is `Some` or any of the default paths if they exist (attempt in order).
pub fn path_or_defaults_if_exist(path: &Option<PathBuf>, defaults: &[PathBuf]) -> Option<PathBuf> {
    let mut path = path.clone();
    for default in defaults {
        if path.is_some() {
            break;
        }
        path = default.exists().then_some(default.clone());
    }
    path
}

/// Loads the wallet context from the given path.
///
/// If no path is provided, tries to load the configuration first from the local folder, and then
/// from the standard Sui configuration directory.
// NB: When making changes to the logic, make sure to update the argument docs in
// `crates/walrus-service/bin/client.rs`.
#[allow(dead_code)]
pub fn load_wallet_context(path: &Option<PathBuf>) -> Result<WalletContext> {
    let mut default_paths = vec!["./client.yaml".into(), "./sui_config.yaml".into()];
    if let Some(home_dir) = home::home_dir() {
        default_paths.push(home_dir.join(".sui").join("sui_config").join("client.yaml"))
    }
    let path = path_or_defaults_if_exist(path, &default_paths)
        .ok_or(anyhow!("Could not find a valid wallet config file."))?;
    tracing::info!("Using wallet configuration from {}", path.display());
    WalletContext::new(&path, None, None)
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

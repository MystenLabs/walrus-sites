// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str,
    time::SystemTime,
};

use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Utc};
use futures::Future;
use glob::Pattern;
use serde::{Deserialize, Deserializer};
use sui_keys::keystore::AccountKeystore;
use sui_sdk::{
    rpc_types::{
        Page,
        SuiExecutionStatus,
        SuiTransactionBlockEffects,
        SuiTransactionBlockEffectsAPI,
        SuiTransactionBlockResponse,
    },
    wallet_context::WalletContext,
};
use sui_types::{
    base_types::{ObjectID, ObjectRef, SuiAddress},
    object::Owner,
    transaction::{ProgrammableTransaction, TransactionData},
    TypeTag,
};
use walrus_core::{BlobId as BlobIdOriginal, QuiltPatchId};

use crate::{
    display,
    retry_client::RetriableSuiClient,
    site::config::WSResources,
    types::{HttpHeaders, ObjectCache, Staking, StakingInnerV1, StakingObjectForDeserialization},
    walrus::{
        output::{EpochCount, EpochTimeOrMessage, InfoEpochOutput, SuiBlob},
        types::BlobId,
    },
};

#[cfg(test)]
#[path = "unit_tests/util.tests.rs"]
mod util_tests;

pub(crate) async fn sign_and_send_ptb(
    active_address: SuiAddress,
    wallet: &WalletContext,
    retry_client: &RetriableSuiClient,
    programmable_transaction: ProgrammableTransaction,
    gas_coin: ObjectRef,
    gas_budget: u64,
    object_cache: &mut ObjectCache,
) -> Result<SuiTransactionBlockResponse> {
    let gas_price = wallet.get_reference_gas_price().await?;
    let transaction = TransactionData::new_programmable(
        active_address,
        vec![gas_coin],
        programmable_transaction,
        gas_budget,
        gas_price,
    );
    let transaction = wallet.sign_transaction(&transaction).await;
    let resp = retry_client.execute_transaction(transaction).await?;
    let digest = resp.digest;
    let effects = resp
        .effects
        .as_ref()
        .ok_or(anyhow!("Expected effects for transaction {}", digest))?;
    update_cache_from_effects(object_cache, effects);
    Ok(resp)
}

/// Updates the object cache with the changed objects from transaction effects.
///
/// Only objects with `AddressOwner` or `ObjectOwner` ownership are cached,
/// as shared and immutable objects don't have version conflicts in the same way.
fn update_cache_from_effects(object_cache: &mut ObjectCache, effects: &SuiTransactionBlockEffects) {
    for obj in effects.all_changed_objects() {
        match obj.0.owner {
            Owner::ObjectOwner(_) | Owner::AddressOwner(_) => {
                object_cache.insert(obj.0.object_id(), obj.0.reference.to_object_ref());
            }
            _ => {}
        }
    }
}

pub(crate) async fn handle_pagination<F, T, C, Fut>(
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
/// Fails if the created site object ID cannot be found in the transaction effects.
/// This can happen if, for example, no object owned by the provided `address` was created
/// in the transaction, or if the transaction did not result in the expected object creation
/// structure that this function relies on. Can also fail if the transaction itself failed (not
/// enough gas, etc.)
pub(crate) fn get_site_id_from_response(
    address: SuiAddress,
    effects: &SuiTransactionBlockEffects,
) -> Result<ObjectID> {
    // Return type changed to ObjectID
    tracing::debug!(
        ?effects,
        "getting the object ID of the created Walrus site."
    );
    if let SuiExecutionStatus::Failure { error } = &effects.status() {
        anyhow::bail!("site ptb failed with error: {error}");
    }
    Ok(effects
        .created()
        .iter()
        .find(|c| {
            c.owner
                .get_owner_address()
                .map(|owner_address| owner_address == address)
                .unwrap_or(false)
        })
        .ok_or(anyhow::anyhow!("failed to get site_id from response"))?
        .reference
        .object_id)
}

/// Returns the path if it is `Some` or any of the default paths if they exist (attempt in order).
pub(crate) fn path_or_defaults_if_exist(
    path: Option<&Path>,
    defaults: &[PathBuf],
) -> Option<PathBuf> {
    let mut path = path.map(|p| p.to_path_buf());
    for default in defaults {
        if path.is_some() {
            break;
        }
        path = default.exists().then_some(default.clone());
    }
    path
}

/// Loads the wallet context from the given optional wallet config (optional path and optional
/// Sui env).
///
/// If no path is provided, tries to load the configuration first from the local folder, and
/// then from the standard Sui configuration directory.
// NB: When making changes to the logic, make sure to update the argument docs in
// `crates/walrus-service/bin/client.rs`.
pub(crate) fn load_wallet_context(
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
pub(crate) fn persist_site_id_and_name(
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

/// Matches a resource path against a glob pattern.
///
/// This function uses glob pattern matching to determine if a resource path matches
/// the given pattern. Glob patterns support wildcard characters for flexible matching.
///
/// # Glob Pattern Syntax
/// - `*` matches zero or more characters (excluding path separators in some contexts)
/// - `?` matches exactly one character
/// - `[abc]` matches any character within the brackets
/// - `[a-z]` matches any character in the specified range
/// - `**` matches zero or more directories (when used as a path component)
///
/// # Arguments
/// * `pattern` - The glob pattern to match against
/// * `resource_path` - The resource path to test
///
/// # Returns
/// Returns `true` if the resource path matches the pattern, `false` otherwise.
/// If the pattern is invalid, returns `false`.
pub(crate) fn is_pattern_match(pattern: &str, resource_path: &str) -> bool {
    Pattern::new(pattern)
        .map(|pattern| pattern.matches(resource_path))
        .expect("Invalid glob pattern.")
}

/// Checks if a resource path matches any of the provided ignore patterns.
pub(crate) fn is_ignored<'a>(
    mut ignore_patterns: impl Iterator<Item = &'a str>,
    resource_path: &str,
) -> bool {
    ignore_patterns.any(|pattern| is_pattern_match(pattern, resource_path))
}

/// Fetches the staking object by its ID and the current walrus package ID.
/// Returns a `StakingObject` that includes version, package IDs, and staking parameters.
pub(crate) async fn get_staking_object(
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

/// Decodes a hexadecimal string (with "0x" prefix) into a vector of bytes.
///
/// # Arguments
/// * `hex_str` - A string slice containing the hexadecimal representation, prefixed with "0x".
///
/// # Returns
/// Returns a `Result` containing the decoded bytes as a `Vec<u8>` on success,
/// or a `String` error message on failure.
pub(crate) fn decode_hex(hex_str: &str) -> Result<Vec<u8>, std::string::String> {
    hex::decode(&hex_str[2..]).map_err(|e| format!("Failed to decode hex: {e}"))
}

/// Parses a QuiltPatchId from a resource header and a BlobId.
///
/// It attempts to construct the internal quilt patch ID from a provided
/// special HTTP header and combines it with the given BlobId.
///
/// # Arguments
/// * `blob_id` - The BlobId associated with the resource.
/// * `resource_headers` - The HTTP headers containing metadata, expected to include
///   "x-wal-quilt-patch-internal-id" as a hex string.
///
/// # Returns
/// Returns `Some(QuiltPatchId)` if the required header is present and valid, otherwise `None`.
pub(crate) fn parse_quilt_patch_id(
    blob_id: &BlobId,
    resource_headers: &HttpHeaders,
) -> Option<QuiltPatchId> {
    let quilt_id =
        BlobIdOriginal::try_from(&blob_id.0[..BlobIdOriginal::LENGTH]).expect("Not valid blob ID");
    resource_headers
        .get("x-wal-quilt-patch-internal-id")
        .map(|patch_id_bytes| {
            QuiltPatchId::new(
                quilt_id,
                decode_hex(patch_id_bytes).expect("Invalid patch id"),
            )
        })
}

#[cfg(test)]
mod parse_quilt_patch_id_tests {
    use std::str::FromStr;

    use super::parse_quilt_patch_id;
    use crate::{
        types::{HttpHeaders, VecMap},
        walrus::types::BlobId,
    };

    #[test]
    /// The values of this test were retrieved by publishing a site as a quilt, and then
    /// inspecting the index.html resource. This way we know exactly what behaviour to expect.
    fn test_parse_quilt_patch_id_happy() {
        // examples/snake/index.html Quilt/Blob ID
        let blob_id = BlobId::from_str("Jqz2KSMu18pygjkC-WVEQqtUZRo18-cuf_566VZSxVo")
            .expect("Invalid blob ID.");
        let mut resource_headers: VecMap<String, String> = VecMap::new();
        // Supposing we published examples/snake/index.html as a quilt patch, we add the
        // hex encoded internal quilt patch identifier in the headers.
        resource_headers.insert(
            "x-wal-quilt-patch-internal-id".to_string(),
            "0x010c001900".to_string(),
        );
        assert!(
            parse_quilt_patch_id(&blob_id, &HttpHeaders(resource_headers))
                // Compare that the resulting quilt patch ID is the b64 encoded blobID + internal patch ID
                .is_some_and(
                    |x| x.to_string() == "Jqz2KSMu18pygjkC-WVEQqtUZRo18-cuf_566VZSxVoBDAAZAA"
                )
        );
    }
}

#[cfg(test)]
mod decode_hex_tests {
    use super::decode_hex;

    #[test]
    fn test_decode_hex_happy_path() {
        let hex_str = "0x48656c6c6f"; // "Hello"
        let result = decode_hex(hex_str);
        assert_eq!(result.unwrap(), b"Hello");
    }

    #[test]
    fn test_decode_hex_expected_failure() {
        let hex_str = "0xZZZZZZ";
        let result = decode_hex(hex_str);
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("Failed to decode hex"));
    }
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

pub fn get_epochs_ahead(
    earliest_expiry_time: SystemTime,
    InfoEpochOutput {
        start_of_current_epoch,
        epoch_duration,
        max_epochs_ahead,
        ..
    }: InfoEpochOutput,
) -> anyhow::Result<EpochCount> {
    let estimated_start_of_current_epoch = match start_of_current_epoch {
        EpochTimeOrMessage::Message(_) => Utc::now(),
        EpochTimeOrMessage::DateTime(start) => start,
    };
    let epoch_duration_millis: u64 = epoch_duration
        .as_millis()
        .try_into()
        .context("epoch duration is too long")?;
    let earliest_expiry_ts: DateTime<Utc> = earliest_expiry_time.into();
    if earliest_expiry_ts < estimated_start_of_current_epoch || earliest_expiry_ts < Utc::now() {
        bail!(
            "earliest_expiry_time must be greater than the current epoch start time and the current time"
        );
    }
    let delta = (earliest_expiry_ts - estimated_start_of_current_epoch).num_milliseconds() as u64;
    let epochs_ahead = (delta / epoch_duration_millis + 1)
        .try_into()
        .map_err(|_| anyhow::anyhow!("expiry time is too far in the future"))?;

    // Check that the number of epochs is lower than the number of epochs the blob can be stored
    // for.
    if epochs_ahead > max_epochs_ahead {
        bail!("blobs can only be stored for up to {max_epochs_ahead} epochs ahead; {epochs_ahead} epochs were requested");
    }

    Ok(epochs_ahead)
}

pub async fn get_owned_blobs(
    sui_client: &RetriableSuiClient,
    walrus_package: ObjectID,
    owner_address: SuiAddress,
) -> anyhow::Result<HashMap<BlobId, (SuiBlob, ObjectRef)>> {
    let type_map = sui_client
        .type_origin_map_for_package(walrus_package)
        .await?;
    let blobs = sui_client
        .get_owned_objects_of_type::<SuiBlob>(owner_address, &type_map, &[])
        .await?
        .map(|blob| (blob.0.blob_id, blob))
        .collect();
    Ok(blobs)
}

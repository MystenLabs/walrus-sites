use std::{fs::read_dir, path::PathBuf, str};

use anyhow::{anyhow, ensure, Result};
use shared_crypto::intent::Intent;
use sui_keys::keystore::{AccountKeystore, Keystore};
use sui_sdk::{
    rpc_types::{
        SuiExecutionStatus, SuiObjectDataOptions, SuiTransactionBlockEffectsAPI,
        SuiTransactionBlockResponse, SuiTransactionBlockResponseOptions,
    },
    SuiClient,
};
use sui_types::{
    base_types::{ObjectID, ObjectRef, SuiAddress},
    quorum_driver_types::ExecuteTransactionRequestType,
    transaction::{ProgrammableTransaction, Transaction, TransactionData},
};

pub fn recursive_readdir(root: &PathBuf) -> Vec<PathBuf> {
    let mut files = vec![];
    let entries = read_dir(root).expect("Reading path failed. Please provide a valid path");
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(recursive_readdir(&path));
        } else {
            files.push(path);
        }
    }
    files
}

pub async fn sign_and_send_ptb(
    client: &SuiClient,
    keystore: &Keystore,
    address: SuiAddress,
    programmable_transaction: ProgrammableTransaction,
    gas_coin: ObjectRef,
    gas_budget: u64,
) -> Result<SuiTransactionBlockResponse> {
    let gas_price = client.read_api().get_reference_gas_price().await?;

    let transaction = TransactionData::new_programmable(
        address,
        vec![gas_coin],
        programmable_transaction,
        gas_budget,
        gas_price,
    );
    let signature = keystore.sign_secure(&address, &transaction, Intent::sui_transaction())?;
    let response = client
        .quorum_driver_api()
        .execute_transaction_block(
            Transaction::from_data(transaction, vec![signature]),
            SuiTransactionBlockResponseOptions::full_content(),
            Some(ExecuteTransactionRequestType::WaitForLocalExecution),
        )
        .await?;
    ensure!(
        response.confirmed_local_execution == Some(true),
        "Transaction execution was not confirmed"
    );
    match response
        .effects
        .as_ref()
        .ok_or_else(|| anyhow!("No transaction effects in response"))?
        .status()
    {
        SuiExecutionStatus::Success => Ok(response),
        SuiExecutionStatus::Failure { error } => {
            Err(anyhow!("Error in transaction execution: {}", error))
        }
    }
}

pub async fn get_object_ref_from_id(client: &SuiClient, id: ObjectID) -> Result<ObjectRef> {
    client
        .read_api()
        .get_object_with_options(id, SuiObjectDataOptions::new())
        .await?
        .object_ref_if_exists()
        .ok_or_else(|| anyhow!("Could not get object reference for object with id {}", id))
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

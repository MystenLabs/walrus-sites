use anyhow::Result;
use sui_keys::keystore::AccountKeystore;
use sui_sdk::rpc_types::SuiTransactionBlockResponse;
use sui_types::{
    base_types::ObjectID,
    programmable_transaction_builder::ProgrammableTransactionBuilder,
    transaction::{CallArg, TransactionData},
    Identifier,
};
use walrus_service::cli_utils::load_wallet_context;

use crate::{
    // util::{call_arg_from_shared_object_id, get_object_ref_from_id, sign_and_send_ptb},
    Config,
};

pub async fn set_suins_name(
    config: Config,
    package: &ObjectID,
    sui_ns: &ObjectID,
    registration: &ObjectID,
    target: &ObjectID,
) -> Result<SuiTransactionBlockResponse> {
    let wallet = load_wallet_context(&config.walrus.wallet_config)?;
    let gas_price = wallet.get_reference_gas_price().await?;

    let mut builder = ProgrammableTransactionBuilder::new();
    tracing::debug!(ns=?sui_ns, "getting the suins argument");
    let suins_arg = builder.input(wallet.get_object_ref(*sui_ns).await?.into())?;
    tracing::debug!(reg=?registration, "getting the registration argument");
    // let reg_arg = builder.input(wallet.get_object_ref(*registration).await?.into())?;
    let reg_arg = builder.pure(vec![registration.into_bytes()])?;

    tracing::debug!("getting the target argument");
    let target_arg = builder.pure(vec![target.into_bytes()])?;
    let clock_arg = builder.input(CallArg::CLOCK_IMM)?;

    tracing::debug!("building ptb");
    builder.programmable_move_call(
        *package,
        Identifier::new("controller").unwrap(),
        Identifier::new("set_target_address").unwrap(),
        vec![],
        vec![suins_arg, reg_arg, target_arg, clock_arg],
    );

    let active_address = wallet.config.active_address.unwrap_or(
        *wallet
            .config
            .keystore
            .addresses()
            .first()
            .expect("running with a wallet"),
    );

    let gas_coin = wallet.get_object_ref(config.gas_coin).await?;

    tracing::debug!("building transaction");
    let transaction = TransactionData::new_programmable(
        active_address,
        vec![gas_coin],
        builder.finish(),
        config.gas_budget,
        gas_price,
    );
    let transaction = wallet.sign_transaction(&transaction);
    wallet.execute_transaction_may_fail(transaction).await
}

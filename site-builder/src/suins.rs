use anyhow::Result;
use sui_sdk::rpc_types::SuiTransactionBlockResponse;
use sui_types::{
    base_types::ObjectID,
    programmable_transaction_builder::ProgrammableTransactionBuilder,
    transaction::CallArg,
    Identifier,
};
use walrus_service::cli_utils::load_wallet_context;

use crate::{
    util::{call_arg_from_shared_object_id, get_object_ref_from_id, sign_and_send_ptb},
    Config,
};

pub async fn set_suins_name(
    config: Config,
    package: &ObjectID,
    sui_ns: &ObjectID,
    registration: &ObjectID,
    target: &ObjectID,
) -> Result<SuiTransactionBlockResponse> {
    let client = load_wallet_context(&config.walrus.wallet_config).get_client().await?;
    let mut builder = ProgrammableTransactionBuilder::new();
    let suins_arg = builder.input(call_arg_from_shared_object_id(&client, *sui_ns, true).await?)?;
    let reg_arg = builder.input(get_object_ref_from_id(&client, *registration).await?.into())?;
    let target_arg = builder.pure(vec![target.into_bytes()])?;
    let clock_arg = builder.input(CallArg::CLOCK_IMM)?;
    builder.programmable_move_call(
        *package,
        Identifier::new("controller").unwrap(),
        Identifier::new("set_target_address").unwrap(),
        vec![],
        vec![suins_arg, reg_arg, target_arg, clock_arg],
    );
    sign_and_send_ptb(
        &client,
        config.network.keystore(),
        config.network.address(),
        builder.finish(),
        get_object_ref_from_id(&client, config.gas_coin).await?,
        config.gas_budget,
    )
    .await
}

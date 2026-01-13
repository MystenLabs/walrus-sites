// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Infrastructure for retrying RPC calls with backoff, in case there are network errors.
//!
//! Wraps the [`SuiClient`] to introduce retries.

use std::{fmt::Debug, future::Future};

use anyhow::{anyhow, bail, Result};
use rand::{
    rngs::{StdRng, ThreadRng},
    Rng as _,
};
use serde::{de::DeserializeOwned, Serialize};
use sui_sdk::{
    apis::EventApi,
    error::SuiRpcResult,
    rpc_types::{
        Balance,
        Coin,
        DynamicFieldInfo,
        ObjectsPage,
        Page,
        SuiObjectData,
        SuiObjectDataFilter,
        SuiObjectDataOptions,
        SuiObjectResponse,
        SuiObjectResponseQuery,
        SuiRawData,
        SuiTransactionBlockResponse,
        SuiTransactionBlockResponseOptions,
    },
    wallet_context::WalletContext,
    SuiClient,
};
use sui_types::{
    base_types::{ObjectID, ObjectRef, ObjectType, SuiAddress},
    dynamic_field::derive_dynamic_field_id,
    quorum_driver_types::ExecuteTransactionRequestType::WaitForLocalExecution,
    transaction::Transaction,
    TypeTag,
};
use tracing::Level;
use walrus_sdk::{
    core_utils::backoff::ExponentialBackoffConfig as WalrusBackoffConfig,
    sui::client::retry_client::RetriableSuiClient as WalrusRetriableSuiClient,
};

use crate::{
    backoff::{BackoffStrategy, ExponentialBackoff, ExponentialBackoffConfig},
    site::contracts::{
        self,
        get_sui_object_from_object_response,
        AssociatedContractStruct,
        TypeOriginMap,
    },
    types::SuiDynamicField,
    util::handle_pagination,
};

/// The list of HTTP status codes that are retriable.
const RETRIABLE_RPC_ERRORS: &[&str] = &["429", "500", "502"];

/// Trait to test if an error is produced by a temporary RPC failure and can be retried.
pub trait RetriableRpcError: Debug {
    /// Returns `true` if the error is a retriable network error.
    fn is_retriable_rpc_error(&self) -> bool;
}

impl RetriableRpcError for anyhow::Error {
    fn is_retriable_rpc_error(&self) -> bool {
        self.downcast_ref::<sui_sdk::error::Error>()
            .map(|error| error.is_retriable_rpc_error())
            .unwrap_or(false)
    }
}

impl RetriableRpcError for sui_sdk::error::Error {
    fn is_retriable_rpc_error(&self) -> bool {
        if let sui_sdk::error::Error::RpcError(rpc_error) = self {
            let error_string = rpc_error.to_string();
            if RETRIABLE_RPC_ERRORS
                .iter()
                .any(|&s| error_string.contains(s))
            {
                return true;
            }
        }
        false
    }
}

/// Retries the given function while it returns retriable errors.[
async fn retry_rpc_errors<S, F, T, E, Fut>(mut strategy: S, mut func: F) -> Result<T, E>
where
    S: BackoffStrategy,
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: RetriableRpcError,
{
    loop {
        let value = func().await;

        match value {
            Ok(value) => return Ok(value),
            Err(error) if error.is_retriable_rpc_error() => {
                if let Some(delay) = strategy.next_delay() {
                    tracing::debug!(
                        ?delay,
                        ?error,
                        "attempt failed with retriable RPC error, waiting before retrying"
                    );
                    tokio::time::sleep(delay).await;
                } else {
                    tracing::debug!(
                        "last attempt failed with retriable RPC error, returning last failure value"
                    );
                    return Err(error);
                }
            }
            Err(error) => {
                tracing::debug!("non-retriable error, returning last failure value");
                return Err(error);
            }
        }
    }
}

/// A [`SuiClient`] that retries RPC calls with backoff in case of network errors.
///
/// This retriable client wraps functions from the [`CoinReadApi`][sui_sdk::apis::CoinReadApi] and
/// the [`ReadApi`][sui_sdk::apis::ReadApi] of the [`SuiClient`], and
/// additionally provides some convenience methods.
#[allow(missing_debug_implementations)]
#[derive(Clone)]
pub struct RetriableSuiClient {
    sui_client: SuiClient,
    backoff_config: ExponentialBackoffConfig,
}

impl RetriableSuiClient {
    /// Creates a new retriable client.
    ///
    /// NB: If you are creating the sui client from a wallet context, you should use
    /// [`RetriableSuiClient::new_from_wallet`] instead. This is because the wallet context will
    /// make a call to the RPC server in [`WalletContext::get_client`], which may fail without any
    /// retries. `new_from_wallet` will handle this case correctly.
    fn new(sui_client: SuiClient, backoff_config: ExponentialBackoffConfig) -> Self {
        RetriableSuiClient {
            sui_client,
            backoff_config,
        }
    }

    /// Returns a reference to the inner backoff configuration.
    #[allow(dead_code)]
    pub fn backoff_config(&self) -> &ExponentialBackoffConfig {
        &self.backoff_config
    }
    /// Returns a reference to the inner SuiClient.
    pub fn client(&self) -> &SuiClient {
        &self.sui_client
    }

    /// Creates a walrus-sdk `RetriableSuiClient` from this client.
    pub fn to_walrus_retriable_client(&self) -> Result<WalrusRetriableSuiClient> {
        WalrusRetriableSuiClient::new(
            vec![self.sui_client.clone().into()],
            WalrusBackoffConfig::default(),
        )
    }

    /// Creates a new retriable client from a wallet context.
    #[tracing::instrument(level = Level::DEBUG, skip_all)]
    pub async fn new_from_wallet(
        wallet: &WalletContext,
        backoff_config: ExponentialBackoffConfig,
    ) -> Result<Self> {
        let strategy = backoff_config.get_strategy(ThreadRng::default().gen());
        let client = retry_rpc_errors(strategy, || async { wallet.get_client().await }).await?;
        Ok(Self::new(client, backoff_config))
    }

    // Reimplementation of the `SuiClient` methods.

    /// Return a list of coins for the given address, or an error upon failure.
    ///
    /// Calls [`sui_sdk::apis::CoinReadApi::select_coins`] internally.
    #[allow(dead_code)]
    #[tracing::instrument(level = Level::DEBUG, skip_all)]
    pub async fn select_coins(
        &self,
        address: SuiAddress,
        coin_type: Option<String>,
        amount: u128,
        exclude: Vec<ObjectID>,
    ) -> SuiRpcResult<Vec<Coin>> {
        retry_rpc_errors(self.get_strategy(), || async {
            self.sui_client
                .coin_read_api()
                .select_coins(address, coin_type.clone(), amount, exclude.clone())
                .await
        })
        .await
    }

    /// Returns the balance for the given coin type owned by address.
    ///
    /// Calls [`sui_sdk::apis::CoinReadApi::get_balance`] internally.
    #[allow(dead_code)]
    #[tracing::instrument(level = Level::DEBUG, skip_all)]
    pub async fn get_balance(
        &self,
        owner: SuiAddress,
        coin_type: Option<String>,
    ) -> SuiRpcResult<Balance> {
        retry_rpc_errors(self.get_strategy(), || async {
            self.sui_client
                .coin_read_api()
                .get_balance(owner, coin_type.clone())
                .await
        })
        .await
    }

    /// Returns the dynamic fields for the object.
    ///
    /// Calls [`sui_sdk::apis::ReadApi::get_dynamic_fields`] internally.
    #[tracing::instrument(level = Level::DEBUG, skip_all)]
    pub async fn get_dynamic_fields(
        &self,
        object_id: ObjectID,
        cursor: Option<ObjectID>,
        limit: Option<usize>,
    ) -> SuiRpcResult<Page<DynamicFieldInfo, ObjectID>> {
        retry_rpc_errors(self.get_strategy(), || async {
            self.sui_client
                .read_api()
                .get_dynamic_fields(object_id, cursor, limit)
                .await
        })
        .await
    }

    /// Gets a single dynamic field by key.
    pub(crate) async fn get_dynamic_field<K, V>(
        &self,
        parent: ObjectID,
        key_type: TypeTag,
        key: K,
    ) -> Result<V>
    where
        K: DeserializeOwned + Serialize,
        V: DeserializeOwned + Serialize,
    {
        let object_id = derive_dynamic_field_id(
            parent,
            &key_type,
            &bcs::to_bytes(&key).expect("key should be serializable"),
        )?;

        tracing::debug!(?object_id, "getting single dynamic field");

        let field: SuiDynamicField<K, V> = self.get_sui_object(object_id).await?;
        Ok(field.value)
    }

    /// Return a paginated response with the objects owned by the given address.
    ///
    /// Calls [`sui_sdk::apis::ReadApi::get_owned_objects`] internally.
    #[tracing::instrument(level = Level::DEBUG, skip_all)]
    pub async fn get_owned_objects(
        &self,
        address: SuiAddress,
        query: Option<SuiObjectResponseQuery>,
        cursor: Option<ObjectID>,
        limit: Option<usize>,
    ) -> SuiRpcResult<ObjectsPage> {
        retry_rpc_errors(self.get_strategy(), || async {
            self.sui_client
                .read_api()
                .get_owned_objects(address, query.clone(), cursor, limit)
                .await
        })
        .await
    }

    /// Returns a [`SuiObjectResponse`] based on the provided [`ObjectID`].
    ///
    /// Calls [`sui_sdk::apis::ReadApi::get_object_with_options`] internally.
    #[tracing::instrument(level = Level::DEBUG, skip_all)]
    pub async fn get_object_with_options(
        &self,
        object_id: ObjectID,
        options: SuiObjectDataOptions,
    ) -> SuiRpcResult<SuiObjectResponse> {
        retry_rpc_errors(self.get_strategy(), || async {
            self.sui_client
                .read_api()
                .get_object_with_options(object_id, options.clone())
                .await
        })
        .await
    }

    /// Returns the original package ID for the given object's type.
    ///
    /// The original package ID is the runtime ID used by Move - the first version of the package,
    /// shared across all upgrades.
    #[tracing::instrument(level = Level::DEBUG, skip_all)]
    pub async fn get_object_original_package(&self, object_id: ObjectID) -> Result<ObjectID> {
        let response = self
            .get_object_with_options(object_id, SuiObjectDataOptions::new().with_type())
            .await?;
        let object_data = response
            .data
            .ok_or_else(|| anyhow!("object {object_id} not found"))?;
        let ObjectType::Struct(move_object_type) = object_data.object_type()? else {
            bail!("object ID ({object_id}) points to a package, not an object");
        };
        Ok(ObjectID::from_address(move_object_type.address()))
    }

    /// Return a list of [SuiObjectResponse] from the given vector of [ObjectID]s.
    ///
    /// Calls [`sui_sdk::apis::ReadApi::multi_get_object_with_options`] internally.
    #[allow(dead_code)]
    #[tracing::instrument(level = Level::DEBUG, skip_all)]
    pub async fn multi_get_object_with_options(
        &self,
        object_ids: Vec<ObjectID>,
        options: SuiObjectDataOptions,
    ) -> SuiRpcResult<Vec<SuiObjectResponse>> {
        retry_rpc_errors(self.get_strategy(), || async {
            self.sui_client
                .read_api()
                .multi_get_object_with_options(object_ids.clone(), options.clone())
                .await
        })
        .await
    }

    /// Returns a reference to the [`EventApi`].
    ///
    /// Internally calls the [`SuiClient::event_api`] function. Note that no retries are
    /// implemented for this function.
    #[allow(dead_code)]
    pub fn event_api(&self) -> &EventApi {
        self.sui_client.event_api()
    }

    // Other wrapper methods.

    #[tracing::instrument(level = Level::DEBUG, skip_all)]
    pub(crate) async fn get_sui_object<U>(&self, object_id: ObjectID) -> Result<U>
    where
        U: AssociatedContractStruct,
    {
        retry_rpc_errors(self.get_strategy(), || async {
            get_sui_object_from_object_response(
                &self
                    .get_object_with_options(
                        object_id,
                        SuiObjectDataOptions::new().with_bcs().with_type(),
                    )
                    .await?,
            )
        })
        .await
    }

    /// Gets the type origin map for a given package.
    pub(crate) async fn type_origin_map_for_package(
        &self,
        package_id: ObjectID,
    ) -> Result<TypeOriginMap> {
        let Ok(Some(SuiRawData::Package(raw_package))) = self
            .get_object_with_options(
                package_id,
                SuiObjectDataOptions::default().with_type().with_bcs(),
            )
            .await?
            .into_object()
            .map(|object| object.bcs)
        else {
            bail!("package not found with ID {package_id}");
        };
        Ok(raw_package
            .type_origin_table
            .into_iter()
            .map(|origin| ((origin.module_name, origin.datatype_name), origin.package))
            .collect())
    }

    /// Gets a backoff strategy, seeded from the internal RNG.
    fn get_strategy(&self) -> ExponentialBackoff<StdRng> {
        self.backoff_config.get_strategy(ThreadRng::default().gen())
    }

    /// Executes a transaction.
    #[tracing::instrument(err, skip(self))]
    pub(crate) async fn execute_transaction(
        &self,
        transaction: Transaction,
    ) -> anyhow::Result<SuiTransactionBlockResponse> {
        // Retry here must use the exact same transaction to avoid locked objects.
        retry_rpc_errors(self.get_strategy(), || async {
            Ok(self
                .sui_client
                .quorum_driver_api()
                .execute_transaction_block(
                    transaction.clone(),
                    SuiTransactionBlockResponseOptions::new()
                        .with_effects()
                        .with_input()
                        .with_events()
                        .with_object_changes()
                        .with_balance_changes(),
                    Some(WaitForLocalExecution),
                )
                .await?)
        })
        .await
    }

    /// Get all the owned objects of the specified type for the specified owner.
    ///
    /// If some of the returned objects cannot be converted to the expected type, they are ignored.
    pub(crate) async fn get_owned_objects_of_type<'a, U>(
        &'a self,
        owner: SuiAddress,
        type_origin_map: &'a TypeOriginMap,
        type_args: &'a [TypeTag],
    ) -> Result<impl Iterator<Item = (U, ObjectRef)> + 'a>
    where
        U: AssociatedContractStruct,
    {
        let results = self
            .get_owned_object_data(owner, type_origin_map, type_args, U::CONTRACT_STRUCT)
            .await?;

        Ok(results.filter_map(|object_data| {
            object_data.map_or_else(
                |error| {
                    tracing::warn!(?error, "failed to convert to local type");
                    None
                },
                |object_data| match U::try_from_object_data(&object_data) {
                    Result::Ok(value) => Some((value, object_data.object_ref())),
                    Result::Err(error) => {
                        tracing::warn!(?error, "failed to convert to local type");
                        None
                    }
                },
            )
        }))
    }

    /// Get all the [`SuiObjectData`] objects of the specified type for the specified owner.
    async fn get_owned_object_data<'a>(
        &'a self,
        owner: SuiAddress,
        type_origin_map: &'a TypeOriginMap,
        type_args: &'a [TypeTag],
        object_type: contracts::StructTag<'a>,
    ) -> Result<impl Iterator<Item = Result<SuiObjectData>> + 'a> {
        let struct_tag =
            object_type.to_move_struct_tag_with_type_map(type_origin_map, type_args)?;
        Ok(handle_pagination(move |cursor| {
            self.get_owned_objects(
                owner,
                Some(SuiObjectResponseQuery {
                    filter: Some(SuiObjectDataFilter::StructType(struct_tag.clone())),
                    options: Some(SuiObjectDataOptions::new().with_bcs().with_type()),
                }),
                cursor,
                None,
            )
        })
        .await?
        .map(|resp: SuiObjectResponse| {
            resp.data.ok_or_else(|| {
                anyhow!(
                    "response does not contain object data [err={:?}]",
                    resp.error
                )
            })
        }))
    }
}

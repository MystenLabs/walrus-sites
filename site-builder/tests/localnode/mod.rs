// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{
    fs::File,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};

use anyhow::{anyhow, bail};
use move_core_types::language_storage::StructTag;
use move_package_alt::schema::Environment;
use serde::Deserialize;
use site_builder::{
    args::GeneralArgs,
    config::Config as SitesConfig,
    contracts,
    contracts::AssociatedContractStruct,
    types::{ResourceDynamicField, SiteFields, SuiResource},
};
use sui_move_build::BuildConfig;
use sui_sdk::{
    rpc_types::{
        ObjectChange,
        SuiData,
        SuiExecutionStatus,
        SuiObjectDataOptions,
        SuiTransactionBlockEffectsAPI,
        SuiTransactionBlockResponseOptions,
        SuiTransactionBlockResponseQuery,
        TransactionFilter,
    },
    SuiClient,
    SuiClientBuilder,
};
use sui_types::{
    base_types::{ObjectID, SuiAddress},
    programmable_transaction_builder::ProgrammableTransactionBuilder,
    transaction::TransactionData,
    transaction_driver_types::ExecuteTransactionRequestType,
    Identifier,
};
use tempfile::TempDir;
use tokio::sync::Mutex as TokioMutex;
use walrus_sdk::{
    core::{
        encoding::{quilt_encoding::QuiltStoreBlob, Primary},
        metadata::QuiltMetadata,
        BlobId,
        QuiltPatchId,
    },
    error::ClientResult,
    node_client::WalrusNodeClient,
    sui::{
        client::{contract_config::ContractConfig, SuiContractClient},
        test_utils::{
            new_wallet_on_sui_test_cluster,
            system_setup::SystemContext,
            TestClusterHandle,
        },
        wallet::Wallet,
    },
};
use walrus_service::test_utils::{test_cluster, StorageNodeHandle, TestCluster};
use walrus_test_utils::WithTempDir;

pub mod args_builder;

// ===== Type definitions for blob epoch testing =====

/// Sui object for storage resources (copied from walrus::output module).
#[derive(Debug, PartialEq, Eq, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestStorageResource {
    /// Object ID of the Sui object.
    pub id: ObjectID,
    /// The start epoch of the resource (inclusive).
    pub start_epoch: u32,
    /// The end epoch of the resource (exclusive).
    pub end_epoch: u32,
    /// The total amount of reserved storage.
    pub storage_size: u64,
}

/// Sui object for a blob (copied from walrus::output::SuiBlob).
#[derive(Debug, PartialEq, Eq, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestBlob {
    /// Object ID of the Sui object.
    pub id: ObjectID,
    /// The epoch in which the blob has been registered.
    pub registered_epoch: u32,
    /// The blob ID.
    pub blob_id: walrus_sdk::core::BlobId,
    /// The (unencoded) size of the blob.
    pub size: u64,
    /// The encoding coding type used for the blob.
    pub encoding_type: u8,
    /// The epoch in which the blob was first certified, `None` if the blob is uncertified.
    pub certified_epoch: Option<u32>,
    /// The [`TestStorageResource`] used to store the blob.
    pub storage: TestStorageResource,
    /// Marks the blob as deletable.
    pub deletable: bool,
}

impl AssociatedContractStruct for TestBlob {
    const CONTRACT_STRUCT: contracts::StructTag<'static> = contracts::walrus::Blob;
}

pub struct WalrusSitesClusterState {
    pub walrus_admin_client: WithTempDir<WalrusNodeClient<SuiContractClient>>,
    pub sui_cluster_handle: Arc<TokioMutex<TestClusterHandle>>,
    pub system_context: SystemContext,
    pub walrus_cluster: TestCluster<StorageNodeHandle>,
    pub walrus_sites_publisher: WithTempDir<Wallet>,
}

pub struct TestSetup {
    pub cluster_state: WalrusSitesClusterState,
    pub client: SuiClient,
    pub sites_config: WithTempDir<(SitesConfig, PathBuf)>,
    pub wallet: WithTempDir<Wallet>,
    pub walrus_config: WithTempDir<(ContractConfig, PathBuf)>,
    pub walrus_sites_package_id: ObjectID,
}

impl TestSetup {
    /// Start test cluster with optional custom epoch duration.
    /// If None, uses the default epoch duration from E2eTestSetupBuilder.
    pub async fn start_local_test_cluster(
        epoch_duration: Option<std::time::Duration>,
    ) -> anyhow::Result<Self> {
        let mut builder = test_cluster::E2eTestSetupBuilder::new();
        if let Some(duration) = epoch_duration {
            builder = builder.with_epoch_duration(duration);
        }
        let (sui_cluster_handle, walrus_cluster, walrus_admin_client, system_context, _) =
            builder.build().await?;
        let rpc_url = sui_cluster_handle.as_ref().lock().await.rpc_url();
        let sui_client = SuiClientBuilder::default().build(rpc_url).await?;

        // ================================= Publish Walrus-Sites ==================================
        let mut walrus_sites_publisher =
            new_wallet_on_sui_test_cluster(sui_cluster_handle.clone()).await?;
        let walrus_sites_package_id =
            publish_walrus_sites(&sui_client, &mut walrus_sites_publisher.inner).await?;

        // ================================= Create walrus config ==================================
        let walrus_sui_client = walrus_admin_client.inner.sui_client();
        let walrus_config = create_walrus_config(walrus_sui_client)?;

        // ========================== Create new wallet and sites config ===========================
        let test_wallet =
            new_wallet_with_sui_and_wal(sui_cluster_handle.clone(), walrus_sui_client).await?;

        let sites_config = create_sites_config(
            test_wallet.inner.get_config_path().to_path_buf(),
            walrus_sites_package_id,
            Some(walrus_config.inner.1.clone()),
        )?;

        Ok(TestSetup {
            cluster_state: WalrusSitesClusterState {
                walrus_admin_client,
                sui_cluster_handle,
                system_context,
                walrus_cluster,
                walrus_sites_publisher,
            },
            client: sui_client,
            sites_config,
            wallet: test_wallet,
            walrus_config,
            walrus_sites_package_id,
        })
    }

    pub async fn read_blob(&self, blob_id: &BlobId) -> ClientResult<Vec<u8>> {
        self.cluster_state
            .walrus_admin_client
            .inner
            .read_blob::<Primary>(blob_id)
            .await
    }

    pub async fn read_quilt_patches<'a>(
        &self,
        quilt_ids: &[QuiltPatchId],
    ) -> ClientResult<Vec<QuiltStoreBlob<'a>>> {
        self.cluster_state
            .walrus_admin_client
            .inner
            .quilt_client()
            .get_blobs_by_ids(quilt_ids)
            .await
    }

    pub async fn read_quilt_patches_by_identifiers<'a>(
        &self,
        blob_id: &BlobId,
        file_identifiers: &[&str],
    ) -> ClientResult<Vec<QuiltStoreBlob<'a>>> {
        self.cluster_state
            .walrus_admin_client
            .inner
            .quilt_client()
            .get_blobs_by_identifiers(blob_id, file_identifiers)
            .await
    }

    pub async fn read_quilt_metadata(&self, quilt_id: &BlobId) -> ClientResult<QuiltMetadata> {
        self.cluster_state
            .walrus_admin_client
            .inner
            .quilt_client()
            .get_quilt_metadata(quilt_id)
            .await
    }

    pub async fn last_site_created(&self) -> anyhow::Result<SiteFields> {
        let resp = self
            .client
            .read_api()
            .query_transaction_blocks(
                SuiTransactionBlockResponseQuery::new_with_filter(
                    TransactionFilter::MoveFunction {
                        package: self.walrus_sites_package_id,
                        module: Some("site".to_string()),
                        function: Some("new_site".to_string()),
                    },
                ),
                None,
                Some(1),
                true,
            )
            .await?;

        let first = resp
            .data
            .first()
            .ok_or(anyhow!("No create site transaction found"))?;
        let resp = self
            .client
            .read_api()
            .get_transaction_with_options(
                first.digest,
                SuiTransactionBlockResponseOptions::new().with_object_changes(),
            )
            .await?;

        let site_id = resp
            .object_changes
            .as_ref()
            .expect("expected object_changes")
            .iter()
            .find_map(|chng| match chng {
                ObjectChange::Created {
                    object_type,
                    object_id,
                    ..
                } if *object_type
                    == StructTag {
                        address: self.walrus_sites_package_id.into(),
                        module: Identifier::from_str("site").unwrap(),
                        name: Identifier::from_str("Site").unwrap(),
                        type_params: vec![],
                    } =>
                {
                    Some(*object_id)
                }
                _ => None,
            })
            .ok_or(anyhow!("Could not find site"))?;

        contracts::get_sui_object(&self.client, site_id).await
    }

    pub async fn site_resources(&self, site_id: ObjectID) -> anyhow::Result<Vec<SuiResource>> {
        let mut resources = vec![];
        let mut has_next = true;
        let mut cursor = None;
        while has_next {
            let dfs = self
                .client
                .read_api()
                .get_dynamic_fields(site_id, cursor, None)
                .await?;
            has_next = dfs.has_next_page;
            cursor = dfs.next_cursor;

            let ids = dfs
                .data
                .into_iter()
                .map(|df| df.object_id)
                .collect::<Vec<ObjectID>>();

            let resource_fields = self
                .client
                .read_api()
                .multi_get_object_with_options(ids, SuiObjectDataOptions::new().with_bcs())
                .await?;

            let mut resources_chunk = resource_fields
                .into_iter()
                .map(|df| {
                    let Some(obj_bcs) = df.data.unwrap().bcs.unwrap().try_into_move() else {
                        return Ok(None);
                    };
                    if !obj_bcs
                        .type_
                        .type_params
                        .first()
                        .unwrap()
                        .to_canonical_string(false)
                        .ends_with("::site::ResourcePath")
                    {
                        return Ok(None);
                    };
                    Ok(Some(
                        bcs::from_bytes::<ResourceDynamicField>(&obj_bcs.bcs_bytes)?.value,
                    ))
                })
                .collect::<anyhow::Result<Vec<Option<SuiResource>>>>()?
                .into_iter()
                .flatten()
                .collect::<Vec<SuiResource>>();
            resources.append(&mut resources_chunk);
        }
        Ok(resources)
    }

    /// Get the current Walrus epoch from the Walrus staking object.
    pub async fn current_walrus_epoch(&self) -> anyhow::Result<u32> {
        let staking_object = self
            .cluster_state
            .walrus_admin_client
            .inner
            .sui_client()
            .read_client
            .get_staking_object()
            .await?;
        Ok(staking_object.epoch())
    }

    /// Get the epoch duration in milliseconds from the Walrus staking object.
    pub async fn epoch_duration_ms(&self) -> anyhow::Result<u64> {
        let staking_object = self
            .cluster_state
            .walrus_admin_client
            .inner
            .sui_client()
            .read_client
            .get_staking_object()
            .await?;
        Ok(staking_object.epoch_duration_millis())
    }

    /// Get the epoch start timestamp from the Walrus staking object.
    /// Returns the estimated start time of the current Walrus epoch.
    pub async fn epoch_start_timestamp(&self) -> anyhow::Result<chrono::DateTime<chrono::Utc>> {
        use walrus_sdk::sui::types::move_structs::EpochState;

        let staking_object = self
            .cluster_state
            .walrus_admin_client
            .inner
            .sui_client()
            .read_client
            .get_staking_object()
            .await?;

        let epoch_state = staking_object.epoch_state();
        let estimated_start_of_current_epoch = match epoch_state {
            EpochState::EpochChangeDone(epoch_start)
            | EpochState::NextParamsSelected(epoch_start) => *epoch_start,
            EpochState::EpochChangeSync(_) => chrono::Utc::now(),
        };

        Ok(estimated_start_of_current_epoch)
    }

    /// Get blob information from a blob object ID.
    /// Returns a `TestBlob` struct with the blob's storage information including end_epoch.
    pub async fn get_blob_info(&self, blob_object_id: ObjectID) -> anyhow::Result<TestBlob> {
        contracts::get_sui_object(&self.client, blob_object_id).await
    }

    /// Get all blob objects owned by the specified address.
    /// Returns a vector of TestBlob structs with their storage information including end_epoch.
    pub async fn get_owned_blobs(
        &self,
        wallet_address: SuiAddress,
    ) -> anyhow::Result<Vec<TestBlob>> {
        let owned_blobs = self
            .client
            .read_api()
            .get_owned_objects(
                wallet_address,
                Some(sui_sdk::rpc_types::SuiObjectResponseQuery::new_with_filter(
                    sui_sdk::rpc_types::SuiObjectDataFilter::StructType(StructTag {
                        address: self.cluster_state.system_context.walrus_pkg_id.into(),
                        module: Identifier::from_str("blob")?,
                        name: Identifier::from_str("Blob")?,
                        type_params: vec![],
                    }),
                )),
                None,
                None,
            )
            .await?;

        let blob_object_ids: Vec<ObjectID> = owned_blobs
            .data
            .into_iter()
            .filter_map(|obj| obj.data.map(|d| d.object_id))
            .collect();

        // Fetch full blob info for each object ID
        let mut blobs = Vec::new();
        for object_id in blob_object_ids {
            let blob = self.get_blob_info(object_id).await?;
            blobs.push(blob);
        }

        Ok(blobs)
    }

    /// Wait for all Walrus nodes to reach the specified epoch.
    pub async fn wait_for_epoch(&self, epoch: u32) {
        self.cluster_state
            .walrus_cluster
            .wait_for_nodes_to_reach_epoch(epoch)
            .await;
    }

    /// Get blob object IDs that were extended via `system::extend_blob` calls.
    ///
    /// This queries all transactions that called `extend_blob` on the Walrus system package,
    /// then extracts the mutated Blob object IDs from those transactions.
    pub async fn get_extended_blob_object_ids(&self) -> anyhow::Result<Vec<ObjectID>> {
        let walrus_pkg_id = self.cluster_state.system_context.walrus_pkg_id;

        // Query transactions that called extend_blob
        let resp = self
            .client
            .read_api()
            .query_transaction_blocks(
                SuiTransactionBlockResponseQuery::new_with_filter(
                    TransactionFilter::MoveFunction {
                        package: walrus_pkg_id,
                        module: Some("system".to_string()),
                        function: Some("extend_blob".to_string()),
                    },
                ),
                None,
                None,
                true,
            )
            .await?;

        let mut blob_object_ids = Vec::new();
        let blob_struct_tag = StructTag {
            address: walrus_pkg_id.into(),
            module: Identifier::from_str("blob")?,
            name: Identifier::from_str("Blob")?,
            type_params: vec![],
        };

        for tx in resp.data {
            let full_resp = self
                .client
                .read_api()
                .get_transaction_with_options(
                    tx.digest,
                    SuiTransactionBlockResponseOptions::new().with_object_changes(),
                )
                .await?;

            if let Some(changes) = full_resp.object_changes {
                for change in changes {
                    if let ObjectChange::Mutated {
                        object_id,
                        object_type,
                        ..
                    } = change
                    {
                        if object_type == blob_struct_tag {
                            blob_object_ids.push(object_id);
                        }
                    }
                }
            }
        }

        Ok(blob_object_ids)
    }

    // ============ Convenient accessors ============

    pub fn sites_config_path(&self) -> &Path {
        self.sites_config.inner.1.as_path()
    }

    pub fn rpc_url(&self) -> &str {
        self.wallet.inner.get_rpc_url()
    }

    pub fn wallet_active_address(&mut self) -> SuiAddress {
        self.wallet.inner.active_address()
    }

    /// Pauses the test and waits for user input, allowing the user to inspect the current
    /// state (e.g., by pasting the fn-url into a Sui explorer in their browser) before continuing.
    pub async fn wait_for_user_input(&mut self) -> anyhow::Result<()> {
        use tokio::io::{self, AsyncBufReadExt, BufReader};
        // Simple readline wait
        let mut stdin = BufReader::new(io::stdin());
        let mut line = String::new();
        println!("FN url: {}", self.rpc_url());
        println!("Wallet address: {}", self.wallet_active_address());
        println!("Press Enter to continue...");
        stdin.read_line(&mut line).await?;
        Ok(())
    }
}

async fn publish_walrus_sites(
    sui_client: &SuiClient,
    publisher: &mut Wallet,
) -> anyhow::Result<ObjectID> {
    const PUBLISH_GAS_BUDGET: u64 = 5_000_000_000;

    // Build package
    let path_buf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("move")
        .join("walrus_site");
    let path = path_buf.as_path();
    let mut move_build_config = BuildConfig::new_for_testing();
    // Use a non-matching environment name so the Move.lock [env.testnet] section
    // doesn't resolve walrus_site to a previously-published address.
    move_build_config.environment = Environment::new("testing".to_string(), "testing".to_string());
    let compiled_modules = move_build_config.build(path)?;
    let modules_bytes = compiled_modules.get_package_bytes(false);

    let wallet_active_address = publisher.active_address();
    let gas_data = sui_client
        .coin_read_api()
        .select_coins(
            wallet_active_address,
            None,
            PUBLISH_GAS_BUDGET as u128,
            vec![],
        )
        .await?;
    let gas_price = sui_client.read_api().get_reference_gas_price().await?;

    // Tx building
    let mut builder = ProgrammableTransactionBuilder::new();
    let upgrade_cap = builder.publish_upgradeable(
        modules_bytes,
        vec![
            ObjectID::from_hex_literal("0x1").unwrap(),
            ObjectID::from_hex_literal("0x2").unwrap(),
        ],
    );
    builder.transfer_arg(wallet_active_address, upgrade_cap);
    let pt = builder.finish();

    let tx_data = TransactionData::new_programmable(
        wallet_active_address,
        gas_data.into_iter().map(|c| c.object_ref()).collect(),
        pt,
        PUBLISH_GAS_BUDGET,
        gas_price,
    );

    let signed_tx = publisher.sign_transaction(&tx_data).await;
    let resp = sui_client
        .quorum_driver_api()
        .execute_transaction_block(
            signed_tx,
            SuiTransactionBlockResponseOptions::default()
                .with_object_changes()
                .with_effects(),
            Some(ExecuteTransactionRequestType::WaitForLocalExecution),
        )
        .await?;

    if let SuiExecutionStatus::Failure { error } = resp
        .effects
        .ok_or(anyhow!(
            "No effects in response. Should publish with show_effects: true"
        ))?
        .status()
    {
        bail!("Publishing walrus sites failed with error:\n{error}");
    };

    resp.object_changes
        .ok_or(anyhow!(
            "No object_changes in response. Should publish with show_object_changes: true"
        ))?
        .into_iter()
        .find_map(|chng| match chng {
            ObjectChange::Published { package_id, .. } => Some(package_id),
            _ => None,
        })
        .ok_or(anyhow!("No published package in response."))
}

pub async fn new_wallet_with_sui_and_wal(
    sui_cluster_handle: Arc<TokioMutex<TestClusterHandle>>,
    walrus_sui_client: &SuiContractClient,
) -> anyhow::Result<WithTempDir<Wallet>> {
    #[allow(clippy::inconsistent_digit_grouping)]
    const WAL_FUND: u64 = 1000_000_000_000;

    let test_wallet = new_wallet_on_sui_test_cluster(sui_cluster_handle.clone()).await?;
    walrus_sui_client
        .send_wal(WAL_FUND, test_wallet.inner.active_address())
        .await?;
    Ok(test_wallet)
}

pub fn create_walrus_config(
    walrus_sui_client: &SuiContractClient,
) -> anyhow::Result<WithTempDir<(ContractConfig, PathBuf)>> {
    let read_client = walrus_sui_client.read_client();
    let walrus_config = read_client.contract_config();
    let temp_dir = TempDir::new().expect("able to create a temporary directory");
    let walrus_config_path = temp_dir
        .path()
        .to_path_buf()
        .join("walrus_client_config.yaml");
    let mut walrus_yaml_file = File::create(walrus_config_path.as_path())?;
    serde_yaml::to_writer(&mut walrus_yaml_file, &walrus_config)?;
    Ok(WithTempDir {
        inner: (walrus_config, walrus_config_path.clone()),
        temp_dir,
    })
}

/// Creates a sites config.
///
/// If `walrus_config_path` is `None`, the walrus config will be discovered from default locations
/// (XDG_CONFIG_HOME/walrus, ~/.config/walrus, ~/.walrus).
pub fn create_sites_config(
    wallet_path: PathBuf,
    walrus_sites_package_id: ObjectID,
    walrus_config_path: Option<PathBuf>,
) -> anyhow::Result<WithTempDir<(SitesConfig, PathBuf)>> {
    let temp_dir = TempDir::new().expect("able to create a temporary directory");
    let sites_config_path = temp_dir.path().to_path_buf().join("sites-config.yaml");

    let sites_config = SitesConfig {
        portal: "".to_string(),
        package: walrus_sites_package_id,
        general: GeneralArgs {
            wallet: Some(wallet_path),
            walrus_config: walrus_config_path,
            ..Default::default()
        },
        staking_object: None,
    };
    let mut file = File::create(sites_config_path.as_path())?;
    serde_yaml::to_writer(&mut file, &sites_config)?;
    Ok(WithTempDir {
        inner: (sites_config, sites_config_path),
        temp_dir,
    })
}

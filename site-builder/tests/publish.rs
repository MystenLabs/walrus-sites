// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

mod localnode;
use std::{fs::File, num::NonZeroU32, path::PathBuf};

use localnode::WalrusSitesClusterState;
use site_builder::{
    args::{
        default,
        Commands,
        EpochArg,
        EpochCountOrMax,
        GeneralArgs,
        PublishOptions,
        WalrusStoreOptions,
    },
    config::Config,
    run,
};
use tempfile::TempDir;
use walrus_test_utils::WithTempDir;

// Important: For tests to pass, the system they are running on need to have walrus installed.
#[tokio::test]
async fn snake() -> anyhow::Result<()> {
    let cluster = WalrusSitesClusterState::new().await?;
    let temp_dir = TempDir::new().expect("able to create a temporary directory");
    let sites_config_path = temp_dir.path().to_path_buf().join("sites-config.yaml");
    let rpc_url = cluster.sui_cluster_handle.lock().await.rpc_url();
    let wallet_path = cluster
        .admin_wallet_with_client
        .temp_dir
        .into_path()
        .join("wallet_config.yaml");
    // println!("rpc_url: {rpc_url}");
    // println!("config.temp_dir: {}", wallet_path.to_str().unwrap());

    let read_client = cluster
        .admin_wallet_with_client
        .inner
        .sui_client()
        .read_client();

    let walrus_config = read_client.contract_config();
    let temp_dir = TempDir::new().expect("able to create a temporary directory");
    let walrus_config_path = temp_dir
        .path()
        .to_path_buf()
        .join("walrus_client_config.yaml");
    println!("walrus_config_path: {walrus_config_path:?}");
    let mut walrus_yaml_file = File::create(walrus_config_path.as_path())?;
    serde_yaml::to_writer(&mut walrus_yaml_file, &walrus_config)?;

    // TODO: This should probably be done in localnode.
    // Config created:
    // ```
    // portal: ''
    // package: 0x1399dde83b06a80b2eb65f4c529596141bb0723411ce8386d8b2fea1c4cf6f28
    // general:
    //   rpc_url: http://127.0.0.1:62139
    //   wallet: /var/folders/94/jrqsqb6s7pl_225b63wygm4m0000gn/T/.tmpXxgwwQ
    //   wallet_env: null
    //   wallet_address: null
    //   walrus_context: null
    //   walrus_binary: walrus
    //   walrus_config: null
    //   walrus_package: null
    //   gas_budget: 500000000
    // staking_object: 0x992a12ab8fe6d1530bed5832c2875064a40d404c53a00357cc61ffd2cbbe8382
    // ```
    let config = WithTempDir {
        inner: Config {
            portal: "".to_string(),
            package: cluster.walrus_sites_package_id,
            general: GeneralArgs {
                rpc_url: Some(rpc_url),
                wallet: Some(wallet_path),
                walrus_config: Some(walrus_config_path),
                ..Default::default()
            },
            staking_object: Some(read_client.get_staking_object_id()),
        },
        temp_dir,
    };

    let mut file = File::create(sites_config_path.as_path())?;
    serde_yaml::to_writer(&mut file, &config.inner)?;

    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("examples")
        .join("snake");
    let ws_resources = directory.join("ws-resources.json");
    run(
        Some(sites_config_path),
        None,
        GeneralArgs::default(),
        Commands::Publish {
            publish_options: PublishOptions {
                directory,
                list_directory: false,
                max_concurrent: None,
                max_parallel_stores: default::max_parallel_stores(),
                walrus_options: WalrusStoreOptions {
                    ws_resources: Some(ws_resources),
                    epoch_arg: EpochArg {
                        epochs: Some(EpochCountOrMax::Epochs(NonZeroU32::new(1).unwrap())),
                        earliest_expiry_time: None,
                        end_epoch: None,
                    },
                    permanent: false,
                    dry_run: false,
                },
            },
            site_name: None,
        },
    )
    .await?;

    Ok(())
}

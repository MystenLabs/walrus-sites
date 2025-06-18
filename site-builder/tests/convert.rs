mod localnode;
use std::{fs::File, time::Duration};

use localnode::WalrusSitesClusterState;
use site_builder::{args::GeneralArgs, config::Config, run};
use sui_types::base_types::ObjectID;
use tempfile::TempDir;
use walrus_test_utils::WithTempDir;

// Important: For tests to pass, the system they are running on need to have walrus installed.
#[tokio::test]
async fn snake() -> anyhow::Result<()> {
    let cluster = WalrusSitesClusterState::new().await?;
    let temp_dir = TempDir::new().expect("able to create a temporary directory");
    let sites_config_path = temp_dir.path().to_path_buf().join("sites-config.yaml");
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
                rpc_url: Some(cluster.sui_cluster_handle.lock().await.rpc_url()),
                wallet: Some(cluster.admin_wallet_with_client.temp_dir.into_path()),
                ..Default::default()
            },
            staking_object: Some(
                cluster
                    .admin_wallet_with_client
                    .inner
                    .sui_client()
                    .read_client()
                    .get_staking_object_id(),
            ),
        },
        temp_dir,
    };
    println!(
        "config.temp_dir: {}",
        config.temp_dir.path().to_str().unwrap()
    );

    let mut file = File::create(sites_config_path.as_path())?;
    serde_yaml::to_writer(&mut file, &config.inner)?;


    run(
        Some(sites_config_path),
        None,
        GeneralArgs::default(),
        site_builder::args::Commands::Convert {
            object_id: ObjectID::random(),
        },
    )
    .await?;
    Ok(())
}

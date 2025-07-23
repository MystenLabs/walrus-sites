use std::{fs, num::NonZeroU32, path::PathBuf, str::FromStr};

use anyhow::{anyhow, bail};
use fastcrypto::hash::{HashFunction, Sha256};
use move_core_types::u256::U256;
use site_builder::args::{Commands, EpochCountOrMax};

#[allow(dead_code)]
mod localnode;
use localnode::{
    args_builder::{ArgsBuilder, PublishOptionsBuilder},
    TestSetup,
};
use sui_sdk::rpc_types::{
    SuiObjectDataOptions,
    SuiTransactionBlockEffectsAPI,
    SuiTransactionBlockResponseOptions,
};
use sui_types::{
    base_types::ObjectRef,
    programmable_transaction_builder::ProgrammableTransactionBuilder,
    transaction::{Command, ObjectArg, TransactionData},
    Identifier,
    TypeTag,
    MOVE_STDLIB_PACKAGE_ID,
};
use walrus_sdk::{
    client::{
        responses::{BlobStoreResult, BlobStoreResultWithPath},
        Client as WalrusSDKClient,
    },
    store_when::StoreWhen,
};
use walrus_sui::{
    client::{BlobPersistence, FixedSystemParameters, ReadClient, SuiContractClient},
    types::Blob,
};

fn get_snake_upload_files(dir: &PathBuf) -> (Vec<PathBuf>, Vec<&str>) {
    let filenames = vec!["Oi-Regular.ttf", "file.svg", "index.html", "walrus.svg"];
    (
        filenames.iter().map(|name| dir.join(name)).collect(),
        filenames,
    )
}

async fn publish_blobs(
    walrus_wallet: &WalrusSDKClient<SuiContractClient>,
    contents: &[(PathBuf, Vec<u8>)],
    epochs: u32,
) -> anyhow::Result<Vec<BlobStoreResultWithPath>> {
    walrus_wallet
        .reserve_and_store_blobs_retry_committees_with_path(
            contents,
            walrus_core::EncodingType::RS2,
            epochs,
            StoreWhen::AlwaysIgnoreResources,
            BlobPersistence::Deletable,
            walrus_sui::client::PostStoreAction::Keep,
        )
        // .reserve_and_register_blobs(1, blobs, walrus_sui::client::BlobPersistence::Deletable)
        .await
        .map_err(anyhow::Error::from)
}

#[tokio::test]
async fn publish_snake() -> anyhow::Result<()> {
    const GAS_BUDGET: u64 = 5_000_000_000;
    let mut cluster = TestSetup::start_local_test_cluster().await?;

    println!("sites-config: {:#?}", &cluster.sites_config);
    println!("sites package_id: {}", &cluster.walrus_sites_package_id);
    println!("other sites package_id: {}", &cluster.other_packages_ids[0]);
    println!(
        "rpc: {}",
        &cluster
            .cluster_state
            .sui_cluster_handle
            .lock()
            .await
            .rpc_url()
    );

    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("examples")
        .join("snake");
    let ws_resources = directory.join("ws-resources.json");

    let sui_balance_pre = cluster
        .client
        .coin_read_api()
        .get_balance(
            cluster
                .walrus_wallet
                .inner
                .sui_client_mut()
                .wallet_mut()
                .active_address()?,
            None,
        )
        .await?
        .total_balance;
    let args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config.inner.1))
        .with_command(Commands::Publish {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory.clone())
                .with_ws_resources(Some(ws_resources))
                .with_epoch_count_or_max(EpochCountOrMax::Epochs(NonZeroU32::try_from(1)?))
                .build()?,
            site_name: None,
        })
        .build()?;
    // println!("args: {args:#?}");

    site_builder::run(args).await?;
    let sui_balance_post = cluster
        .client
        .coin_read_api()
        .get_balance(
            cluster
                .walrus_wallet
                .inner
                .sui_client_mut()
                .wallet_mut()
                .active_address()?,
            None,
        )
        .await?
        .total_balance;
    println!(
        "Used {sui_balance_pre} - {sui_balance_post} = {} MIST",
        sui_balance_pre - sui_balance_post
    );

    let FixedSystemParameters { n_shards, .. } = cluster
        .walrus_wallet
        .inner
        .sui_client()
        .read_client()
        .fixed_system_parameters()
        .await?;
    cluster.cluster_state.walrus_cluster.n_shards = u16::from(n_shards) as usize;

    let (files, names) = get_snake_upload_files(&directory);
    let contents = files
        .into_iter()
        .map(|f| {
            let c = fs::read(f.as_path())?;
            Ok((f, c))
        })
        .collect::<std::io::Result<Vec<(PathBuf, Vec<u8>)>>>()?;
    let hashes = contents.iter().map(|(_, c)| {
        let mut hash_function = Sha256::default();
        hash_function.update(c);
        let blob_hash: [u8; 32] = hash_function.finalize().digest;
        blob_hash
    });
    let blobs = publish_blobs(&cluster.walrus_wallet.inner, &contents, 1).await?;
    // IMPORTANT NOTE: Here we assume that the above command returns blobs in the order that we
    // passed the data.
    // TODO: Get path from BlobStoreResultWithPath
    let names_blobs_and_hashes: Vec<(&str, Blob, [u8; 32])> = names
        .into_iter()
        .zip(blobs.into_iter())
        .zip(hashes.into_iter())
        .map(|((n, b), h)| {
            let BlobStoreResult::NewlyCreated { blob_object, .. } = b.blob_store_result else {
                bail!("Expected NewlyCreated");
            };
            Ok((n, blob_object, h))
        })
        .collect::<anyhow::Result<Vec<(&str, Blob, [u8; 32])>>>()?;

    // tokio::time::sleep(Duration::from_secs(1000)).await;

    let blob_refs = cluster
        .client
        .read_api()
        .multi_get_object_with_options(
            names_blobs_and_hashes.iter().map(|b| b.1.id).collect(),
            SuiObjectDataOptions::new(),
        )
        .await?
        .into_iter()
        .map(|obj_resp| obj_resp.object_ref_if_exists())
        .collect::<Option<Vec<ObjectRef>>>()
        .ok_or(anyhow!("Expected owned object ref"))?;

    // =================== Build ptbs =================
    //
    let mut ptb = ProgrammableTransactionBuilder::new();

    let args = vec![
        ptb.pure(Some("https://docs.wal.app".to_string()))?,
        ptb.pure(Some("https://www.walrus.xyz/walrus-site".to_string()))?,
        ptb.pure(Some("This is a walrus site.".to_string()))?,
        ptb.pure(Some(
            "https://github.com/MystenLabs/walrus-sites/".to_string(),
        ))?,
        ptb.pure(Some("MystenLabs".to_string()))?,
    ];
    let metadata = ptb.command(Command::move_call(
        cluster.other_packages_ids[0],
        Identifier::from_str("metadata_")?,
        Identifier::from_str("new_metadata")?,
        vec![],
        args,
    ));
    let site_name = ptb.pure("Walrus Snake Game")?;

    let site = ptb.command(Command::move_call(
        cluster.other_packages_ids[0],
        Identifier::from_str("site")?,
        Identifier::from_str("new_site")?,
        vec![],
        vec![site_name, metadata],
    ));

    // Add blobs
    for b in blob_refs {
        let obj_arg = ptb.obj(ObjectArg::ImmOrOwnedObject(b))?;
        ptb.command(Command::move_call(
            cluster.other_packages_ids[0],
            Identifier::from_str("facade_1")?,
            Identifier::from_str("add_blob")?,
            vec![],
            vec![site, obj_arg],
        ));
    }

    // Add resources
    names_blobs_and_hashes
        .into_iter()
        .map(|(path, blob, hash)| {
            // option<Range>::none()
            let range = ptb.command(Command::move_call(
                MOVE_STDLIB_PACKAGE_ID,
                Identifier::from_str("option")?,
                Identifier::from_str("none")?,
                vec![TypeTag::from_str(&format!(
                    "{}::resource::Range",
                    cluster.other_packages_ids[0]
                ))?],
                vec![],
            ));

            let path_arg = ptb.pure(path.to_string())?;
            let blob_id = ptb.pure(U256::from_le_bytes(&blob.blob_id.0))?;
            let hash = ptb.pure(U256::from_le_bytes(&hash))?;
            let resource = ptb.command(Command::move_call(
                cluster.other_packages_ids[0],
                Identifier::from_str("resource")?,
                Identifier::from_str("new_resource")?,
                vec![],
                vec![path_arg, blob_id, hash, range],
            ));

            let content_encoding_key = ptb.pure("content-encoding")?;
            let content_encoding_value = ptb.pure("identity")?;
            ptb.command(Command::move_call(
                cluster.other_packages_ids[0],
                Identifier::from_str("resource")?,
                Identifier::from_str("add_header")?,
                vec![],
                vec![resource, content_encoding_key, content_encoding_value],
            ));
            let content_type_key = ptb.pure("content-type")?;
            if path.ends_with(".svg") {
                let svg_content_type_value = ptb.pure("image/svg+xml")?;
                ptb.command(Command::move_call(
                    cluster.other_packages_ids[0],
                    Identifier::from_str("resource")?,
                    Identifier::from_str("add_header")?,
                    vec![],
                    vec![resource, content_type_key, svg_content_type_value],
                ));
            }
            if path.ends_with(".html") {
                let html_content_type_value = ptb.pure("text/html")?;
                ptb.command(Command::move_call(
                    cluster.other_packages_ids[0],
                    Identifier::from_str("resource")?,
                    Identifier::from_str("add_header")?,
                    vec![],
                    vec![resource, content_type_key, html_content_type_value],
                ));
            }
            if path.ends_with(".ttf") {
                let ttf_content_type_value = ptb.pure("font/ttf")?;
                ptb.command(Command::move_call(
                    cluster.other_packages_ids[0],
                    Identifier::from_str("resource")?,
                    Identifier::from_str("add_header")?,
                    vec![],
                    vec![resource, content_type_key, ttf_content_type_value],
                ));
            }

            if path.ends_with(".svg") {
                let svg_cache_key = ptb.pure("Cache-Control")?;
                let svg_cache_value = ptb.pure("public, max-age=86400")?;
                let svg_etag_key = ptb.pure("ETag")?;
                let svg_etag_value = ptb.pure("\"abc123\"")?;
                ptb.command(Command::move_call(
                    cluster.other_packages_ids[0],
                    Identifier::from_str("resource")?,
                    Identifier::from_str("add_header")?,
                    vec![],
                    vec![resource, svg_cache_key, svg_cache_value],
                ));
                ptb.command(Command::move_call(
                    cluster.other_packages_ids[0],
                    Identifier::from_str("resource")?,
                    Identifier::from_str("add_header")?,
                    vec![],
                    vec![resource, svg_etag_key, svg_etag_value],
                ));
            } else if path.ends_with("index.html") {
                let index_cache_key = ptb.pure("Cache-Control")?;
                let index_cache_value = ptb.pure("max-age=3500")?;
                let index_content_key = ptb.pure("Content-Type")?;
                let index_content_value = ptb.pure("text/html; charset=utf-8")?;
                ptb.command(Command::move_call(
                    cluster.other_packages_ids[0],
                    Identifier::from_str("resource")?,
                    Identifier::from_str("add_header")?,
                    vec![],
                    vec![resource, index_cache_key, index_cache_value],
                ));
                ptb.command(Command::move_call(
                    cluster.other_packages_ids[0],
                    Identifier::from_str("resource")?,
                    Identifier::from_str("add_header")?,
                    vec![],
                    vec![resource, index_content_key, index_content_value],
                ));
            }

            ptb.command(Command::move_call(
                cluster.other_packages_ids[0],
                Identifier::from_str("facade_1")?,
                Identifier::from_str("add_resource")?,
                vec![],
                vec![site, resource],
            ));
            Ok(())
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    // Add routes
    let from = ptb.pure("/path/*")?;
    let to = ptb.pure("/file.svg")?;
    ptb.command(Command::move_call(
        cluster.other_packages_ids[0],
        Identifier::from_str("facade_1")?,
        Identifier::from_str("add_route")?,
        vec![],
        vec![site, from, to],
    ));

    let active_addr = cluster.walrus_wallet.inner.sui_client().address();
    ptb.transfer_arg(active_addr, site);

    let ptb = ptb.finish();

    let gas_price = cluster.client.read_api().get_reference_gas_price().await?;
    let gas_coins = cluster
        .client
        .coin_read_api()
        .select_coins(active_addr, None, GAS_BUDGET as u128, vec![])
        .await?
        .into_iter()
        .map(|c| c.object_ref())
        .collect();
    let transaction =
        TransactionData::new_programmable(active_addr, gas_coins, ptb, GAS_BUDGET, gas_price);
    let transaction = cluster
        .walrus_wallet
        .inner
        .sui_client_mut()
        .wallet_mut()
        .sign_transaction(&transaction);
    let resp = cluster
        .client
        .quorum_driver_api()
        .execute_transaction_block(
            transaction,
            SuiTransactionBlockResponseOptions::default().with_effects(),
            None,
        )
        .await?;

    println!("digest: {}", resp.digest);
    println!(
        "{}",
        resp.effects
            .as_ref()
            .expect("with_effects()")
            .status()
            .to_string()
    );

    assert!(resp.effects.expect("with_effects()").status().is_ok());

    let sui_balance_new_post = cluster
        .client
        .coin_read_api()
        .get_balance(
            cluster
                .walrus_wallet
                .inner
                .sui_client_mut()
                .wallet_mut()
                .active_address()?,
            None,
        )
        .await?
        .total_balance;

    println!(
        "Used {sui_balance_post} - {sui_balance_new_post} = {} MIST",
        sui_balance_post - sui_balance_new_post
    );
    // tokio::time::sleep(std::time::Duration::from_secs(1000)).await;

    Ok(())
}

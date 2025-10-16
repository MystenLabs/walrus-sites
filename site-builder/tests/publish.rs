// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;

use fastcrypto::hash::{HashFunction, Sha256};
use move_core_types::u256::U256;
use site_builder::args::{Commands, EpochCountOrMax};
use walrus_sdk::core::BlobId;

#[allow(dead_code)]
mod localnode;
use localnode::{
    args_builder::{ArgsBuilder, PublishOptionsBuilder},
    TestSetup,
};

#[allow(dead_code)]
mod helpers;
use helpers::copy_dir;

#[tokio::test]
#[ignore]
async fn publish_snake() -> anyhow::Result<()> {
    const SNAKE_FILES_UPLOAD_FILES: usize = 4;
    let cluster = TestSetup::start_local_test_cluster().await?;
    let snake_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("examples")
        .join("snake");

    // Copy the entire snake directory to a temp location to avoid modifying the original
    let temp_dir = tempfile::tempdir()?;
    let directory = temp_dir.path().join("snake");
    copy_dir(&snake_dir, &directory)?;

    let args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Publish {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory)
                .with_epoch_count_or_max(EpochCountOrMax::Max)
                .build()?,
            site_name: None,
        })
        .build()?;
    site_builder::run(args).await?;

    let site = cluster.last_site_created().await?;
    let resources = cluster.site_resources(*site.id.object_id()).await?;

    assert_eq!(resources.len(), SNAKE_FILES_UPLOAD_FILES);

    for resource in resources {
        let data = cluster.read_blob(&BlobId(resource.blob_id.0)).await?;
        let mut hash_function = Sha256::default();
        hash_function.update(&data);
        let resource_hash: [u8; 32] = hash_function.finalize().digest;
        assert_eq!(resource.blob_hash, U256::from_le_bytes(&resource_hash));
    }

    Ok(())
}

// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{fs::File, io::Write};

use site_builder::args::{Commands, EpochCountOrMax, ResourceArg, WalrusStoreOptions};

#[allow(dead_code)]
mod localnode;
use localnode::{
    args_builder::{ArgsBuilder, PublishOptionsBuilder},
    TestSetup,
};

#[allow(dead_code)]
mod helpers;
use helpers::{create_test_site, verify_resource_and_get_content};

#[tokio::test]
#[ignore]
async fn test_update_resources_add_files() -> anyhow::Result<()> {
    let cluster = TestSetup::start_local_test_cluster().await?;

    // 1. Generate a dummy site with initial files
    let temp_dir = create_test_site(3)?;
    let directory = temp_dir.path().to_path_buf();

    // Publish the initial site
    let publish_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::Publish {
            publish_options: PublishOptionsBuilder::default()
                .with_directory(directory.clone())
                .with_epoch_count_or_max(EpochCountOrMax::Epochs(1_u32.try_into().unwrap()))
                .build()?,
            site_name: None,
        })
        .build()?;
    site_builder::run(publish_args).await?;

    let site_id = *cluster.last_site_created().await?.id.object_id();

    // 2. Create two new files to add via update-resources
    let new_file1_path = directory.join("new_file1.html");
    let mut new_file1 = File::create(&new_file1_path)?;
    writeln!(new_file1, "<html><body><h1>New File 1</h1></body></html>")?;
    drop(new_file1);

    let new_file2_path = directory.join("new_file2.html");
    let mut new_file2 = File::create(&new_file2_path)?;
    writeln!(new_file2, "<html><body><h1>New File 2</h1></body></html>")?;
    drop(new_file2);

    // 3. Call update-resources to add the two new files
    let update_resources_args = ArgsBuilder::default()
        .with_config(Some(cluster.sites_config_path().to_owned()))
        .with_command(Commands::UpdateResources {
            resources: vec![
                ResourceArg(new_file1_path.clone(), "/new_file1.html".to_string()),
                ResourceArg(new_file2_path.clone(), "/new_file2.html".to_string()),
            ],
            site_object: site_id,
            common: WalrusStoreOptions {
                ws_resources: None,
                epoch_arg: site_builder::args::EpochArg {
                    epochs: Some(EpochCountOrMax::Epochs(1_u32.try_into().unwrap())),
                    earliest_expiry_time: None,
                    end_epoch: None,
                },
                permanent: false,
                dry_run: false,
                max_quilt_size: Default::default(),
            },
        })
        .build()?;
    site_builder::run(update_resources_args).await?;

    // 4. Verify that all files including the added ones are present and valid
    let updated_resources = cluster.site_resources(site_id).await?;

    // Should have 5 files now (3 original + 2 new)
    assert_eq!(
        updated_resources.len(),
        5,
        "Expected 5 resources after adding two"
    );

    // Verify that all resources have valid hashes
    for resource in &updated_resources {
        let _data = verify_resource_and_get_content(&cluster, resource).await?;
    }

    // Verify the new files are present
    let new_resource = updated_resources
        .iter()
        .find(|r| r.path == "/new_file1.html")
        .expect("New resource should be present");

    let new_content = verify_resource_and_get_content(&cluster, new_resource).await?;
    let new_content_str = String::from_utf8(new_content)?;
    assert!(
        new_content_str.contains("New File 1"),
        "New file content should match"
    );
    let new_resource = updated_resources
        .iter()
        .find(|r| r.path == "/new_file2.html")
        .expect("New resource should be present");

    let new_content = verify_resource_and_get_content(&cluster, new_resource).await?;
    let new_content_str = String::from_utf8(new_content)?;
    assert!(
        new_content_str.contains("New File 2"),
        "New file content should match"
    );

    Ok(())
}

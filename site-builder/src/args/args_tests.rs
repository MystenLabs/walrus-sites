// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{num::NonZeroU32, path::PathBuf, str::FromStr};

use sui_types::base_types::ObjectID;

use super::{Args, Commands, GeneralArgs};
use crate::args::{default, EpochArg, EpochCountOrMax, PublishOptions, WalrusStoreOptions};

#[test]
fn test_json_hoist() -> anyhow::Result<()> {
    const WALRUS_PACKAGE: &str =
        "0xfdc88f7d7cf30afab2f82e8380d11ee8f70efb90e863d1de8616fae1bb09ea77";

    let general_arg_inside = Args {
        general: GeneralArgs::default(),
        command: Commands::Json {
            command_string: Some(format!(
                r#"
{{
    "command": {{
        "sitemap": {{
            "siteToMap": "My Walrus Site",
            "walrusPackage": "{WALRUS_PACKAGE}"
        }}
    }}
}}"#
            )),
        },
    };

    let general_arg_outside = Args {
        general: GeneralArgs::default(),
        command: Commands::Json {
            command_string: Some(format!(
                r#"
{{
    "walrusPackage": "{WALRUS_PACKAGE}",
    "command": {{
        "sitemap": {{
            "siteToMap": "My Walrus Site"
        }}
    }}
}}"#
            )),
        },
    };

    let general = GeneralArgs {
        walrus_package: Some(ObjectID::from_str(WALRUS_PACKAGE)?),
        ..Default::default()
    };
    let parsed_inside = general_arg_inside.extract_json_if_present()?;
    let parsed_outside = general_arg_outside.extract_json_if_present()?;
    let expected = Args {
        general,
        command: Commands::Sitemap {
            site_to_map: crate::args::ObjectIdOrName::Name("My Walrus Site".to_string()),
        },
    };

    assert_eq!(parsed_inside, parsed_outside);
    assert_eq!(parsed_inside, expected);
    Ok(())
}

#[test]
fn test_json_publish_parse_all_fields() -> anyhow::Result<()> {
    use std::{num::NonZeroUsize, path::PathBuf};

    use super::{EpochArg, EpochCountOrMax, PublishOptions, WalrusStoreOptions};

    let json = r#"{
        "config": "/path/to/sites_config.yaml",
        "context": "testnet",
        "rpcUrl": "https://rpc.example",
        "gasBudget": 123,
        "command": {
            "publish": {
                "directory": "/tmp/site",
                "listDirectory": true,
                "maxConcurrent": 10,
                "maxParallelStores": 25,
                "wsResources": "/tmp/site/ws-resources.json",
                "epochs": "max",
                "permanent": true,
                "dryRun": true
            }
        }
    }"#;

    let args = Args {
        general: GeneralArgs::default(),
        command: Commands::Json {
            command_string: Some(json.to_string()),
        },
    };

    let parsed = args.extract_json_if_present()?;

    let expected_general = GeneralArgs {
        config: Some(PathBuf::from("/path/to/sites_config.yaml")),
        context: Some("testnet".to_string()),
        rpc_url: Some("https://rpc.example".to_string()),
        gas_budget: Some(123),
        ..Default::default()
    };

    let expected_publish_options = PublishOptions {
        directory: PathBuf::from("/tmp/site"),
        list_directory: true,
        max_concurrent: Some(NonZeroUsize::new(10).unwrap()),
        max_parallel_stores: NonZeroUsize::new(25).unwrap(),
        walrus_options: WalrusStoreOptions {
            ws_resources: Some(PathBuf::from("/tmp/site/ws-resources.json")),
            epoch_arg: EpochArg {
                epochs: Some(EpochCountOrMax::Max),
                earliest_expiry_time: None,
                end_epoch: None,
            },
            permanent: true,
            dry_run: true,
        },
    };

    let expected = Args {
        general: expected_general,
        command: Commands::Publish {
            publish_options: expected_publish_options,
            site_name: None,
        },
    };

    println!("parsed: {parsed:#?}");
    println!("expected: {expected:#?}");
    assert_eq!(parsed, expected);
    Ok(())
}

#[test]
fn test_json_nested_general_overrides_top_level() -> anyhow::Result<()> {
    use std::path::PathBuf;
    let json = r#"{
        "gasBudget": 1,
        "command": {
            "publish": {
                "directory": "/tmp/dir",
                "epochs": 1,
                "gasBudget": 2
            }
        }
    }"#;

    let args = Args {
        general: GeneralArgs::default(),
        command: Commands::Json {
            command_string: Some(json.to_string()),
        },
    };

    let parsed = args.extract_json_if_present()?;

    // gasBudget inside the command should override the top-level one
    assert_eq!(parsed.general.gas_budget, Some(2));

    // Ensure the command was parsed
    match parsed.command {
        Commands::Publish {
            publish_options,
            site_name,
        } => {
            assert_eq!(publish_options.directory, PathBuf::from("/tmp/dir"));
            assert!(site_name.is_none());
        }
        _ => panic!("expected Publish command"),
    }

    Ok(())
}

#[test]
fn test_json_command_nesting_multiple_levels() -> anyhow::Result<()> {
    // Final JSON to be reached after unwrapping nested json commands twice
    let final_json = r#"{
        "gasBudget": 42,
        "command": { "publish": { "directory": "/d", "epochs": 1 } }
    }"#;

    // Escape quotes for embedding inside commandString strings
    let escaped_final = final_json.replace('"', "\\\"");
    let nested_once =
        format!(r#"{{ "command": {{ "json": {{ "commandString": "{escaped_final}" }} }} }}"#,);
    let escaped_once = nested_once.replace('\\', "\\\\");
    let escaped_once = escaped_once.replace('"', "\\\"");
    let nested_twice =
        format!(r#"{{ "command": {{ "json": {{ "commandString": "{escaped_once}" }} }} }}"#,);

    println!("{nested_twice}");
    let args = Args {
        general: GeneralArgs::default(),
        command: Commands::Json {
            command_string: Some(nested_twice),
        },
    };

    let parsed = args.extract_json_if_present()?;
    let expected = Args {
        general: GeneralArgs {
            gas_budget: Some(42),
            ..Default::default()
        },
        command: Commands::Publish {
            publish_options: PublishOptions {
                directory: PathBuf::from_str("/d")?,
                walrus_options: WalrusStoreOptions {
                    epoch_arg: EpochArg {
                        epochs: Some(EpochCountOrMax::Epochs(NonZeroU32::new(1).unwrap())),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                list_directory: false,
                max_concurrent: None,
                max_parallel_stores: default::max_parallel_stores(),
            },
            site_name: None,
        },
    };
    // Ensure we ended up with the innermost command and the hoisted general arg
    assert_eq!(parsed, expected);
    Ok(())
}

#[test]
fn test_json_sitemap_target_parsing_at_name_and_object_id() -> anyhow::Result<()> {
    use super::ObjectIdOrName;

    const WALRUS_PACKAGE: &str =
        "0xfdc88f7d7cf30afab2f82e8380d11ee8f70efb90e863d1de8616fae1bb09ea77";

    // Using @name should normalize to name.sui
    let json_at_name = format!(
        r#"{{
        "walrusPackage": "{WALRUS_PACKAGE}",
        "command": {{ "sitemap": {{ "siteToMap": "@myname" }} }}
    }}"#,
    );

    let args_at = Args {
        general: GeneralArgs::default(),
        command: Commands::Json {
            command_string: Some(json_at_name),
        },
    };
    let parsed_at = args_at.extract_json_if_present()?;
    let expected_general = GeneralArgs {
        walrus_package: Some(ObjectID::from_str(WALRUS_PACKAGE)?),
        ..Default::default()
    };
    let expected_at = Args {
        general: expected_general,
        command: Commands::Sitemap {
            site_to_map: ObjectIdOrName::Name("myname.sui".to_string()),
        },
    };
    assert_eq!(parsed_at, expected_at);

    // Using an object id should parse as ObjectId
    let json_oid = format!(
        r#"{{
        "walrusPackage": "{WALRUS_PACKAGE}",
        "command": {{ "sitemap": {{ "siteToMap": "{WALRUS_PACKAGE}" }} }}
    }}"#,
    );
    let args_oid = Args {
        general: GeneralArgs::default(),
        command: Commands::Json {
            command_string: Some(json_oid),
        },
    };
    let parsed_oid = args_oid.extract_json_if_present()?;
    match parsed_oid.command {
        Commands::Sitemap { site_to_map } => match site_to_map {
            ObjectIdOrName::ObjectId(obj) => assert_eq!(obj, ObjectID::from_str(WALRUS_PACKAGE)?),
            _ => panic!("expected ObjectId variant"),
        },
        _ => panic!("expected Sitemap command"),
    }

    Ok(())
}

#[test]
fn test_json_invalid_missing_command() {
    let json = "{}";
    let args = Args {
        general: GeneralArgs::default(),
        command: Commands::Json {
            command_string: Some(json.to_string()),
        },
    };

    assert!(args.extract_json_if_present().is_err());
}

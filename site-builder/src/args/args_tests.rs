// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use sui_types::base_types::ObjectID;

use super::{Args, Commands, GeneralArgs};

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

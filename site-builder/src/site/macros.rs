// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

macro_rules! match_for_correct_type {
    ($value:expr, $field_type:path) => {
        match $value {
            Some($field_type(x)) => Some(x),
            _ => None,
        }
    };
    ($value:expr, $field_type:path { $var:ident }) => {
        match $value {
            Some($field_type { $var }) => Some($var),
            _ => None,
        }
    };
}

macro_rules! get_dynamic_field {
    ($struct:expr, $field_name:expr, $field_type:path $({ $var:ident })*) => {
        match_for_correct_type!(
            // TODO(mlegner): Change this back to $struct.field_value($field_name) when bumping Sui
            // to a version that includes https://github.com/MystenLabs/sui/pull/18193.
            match &$struct {
                sui_sdk::rpc_types::SuiMoveStruct::WithFields(fields) => {
                    fields.get($field_name).cloned()
                }
                sui_sdk::rpc_types::SuiMoveStruct::WithTypes { type_: _, fields } => {
                    fields.get($field_name).cloned()
                }
                _ => None,
            },
            $field_type $({ $var })*
        ).ok_or(anyhow!(
            "SuiMoveStruct does not contain field {} with expected type {}: {:?}",
            $field_name,
            stringify!($field_type),
            $struct,
        ))
    };
}

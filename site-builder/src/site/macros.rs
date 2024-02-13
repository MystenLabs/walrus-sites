macro_rules! match_for_correct_type {
    ($value: expr, $field_type: path) => {
        match $value {
            Some($field_type(x)) => Some(x),
            _ => None,
        }
    };
    ($value: expr, $field_type: path { $var: ident }) => {
        match $value {
            Some($field_type { $var }) => Some($var),
            _ => None,
        }
    };
}

macro_rules! get_dynamic_field {
    ($struct: expr, $field_name: expr, $field_type: path $({ $var: ident })*) => {
        match_for_correct_type!($struct.read_dynamic_field_value($field_name), $field_type $({ $var })*).ok_or(anyhow!(
            "SuiMoveStruct does not contain field {} with expected type {}: {:?}",
            $field_name,
            stringify!($field_type),
            $struct,
        ))?
    };
}

pub(crate) use get_dynamic_field;

use hyperpuzzle_core::{GeneratorParam, GeneratorParamType, GeneratorParamValue};

use super::*;

/// Parses a Lua specification for a parameter.
pub fn param_from_lua(lua: &Lua, value: LuaValue) -> LuaResult<GeneratorParam> {
    let table: LuaTable = lua.unpack(value)?;

    let name: String;
    let r#type: String;
    let default: LuaValue;
    let min: Option<i64>;
    let max: Option<i64>;
    unpack_table!(lua.unpack(table {
        name,
        r#type,
        default,

        min,
        max,
    }));

    let ty = match r#type.as_str() {
        "int" => {
            let min = min.ok_or_else(|| LuaError::external("`int` type requires `min`"))?;
            let max = max.ok_or_else(|| LuaError::external("`int` type requires `max`"))?;
            GeneratorParamType::Int { min, max }
        }
        s => return Err(LuaError::external(format!("unknown parameter type {s:?}"))),
    };

    let default = param_value_from_lua(lua, &ty, &name, default)?;

    Ok(GeneratorParam { name, ty, default })
}

/// Converts a parameter value into a Lua value.
pub fn param_value_into_lua(lua: &Lua, value: &GeneratorParamValue) -> LuaResult<LuaValue> {
    match value {
        GeneratorParamValue::Int(i) => i.into_lua(lua),
    }
}

/// Converts a Lua value to a value for this parameter and returns an error if
/// it is invalid.
pub fn param_value_from_lua(
    lua: &Lua,
    ty: &GeneratorParamType,
    name: &str,
    value: LuaValue,
) -> LuaResult<GeneratorParamValue> {
    match ty {
        GeneratorParamType::Int { min, max } => {
            let i = i64::from_lua(value, lua)?;
            if i > *max {
                return Err(LuaError::external(format!(
                    "value {i:?} for parameter {name:?} is greater than {max}"
                )));
            }
            if i < *min {
                return Err(LuaError::external(format!(
                    "value {i:?} for parameter {name:?} is less than {min}"
                )));
            }
            Ok(GeneratorParamValue::Int(i))
        }
    }
}

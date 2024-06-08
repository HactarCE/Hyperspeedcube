use std::str::FromStr;

use itertools::Itertools;

use super::*;

/// Conversion wrapper for a string or integer specifying a component of a
/// vector.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct LuaVectorIndex(pub u8);

impl<'lua> FromLua<'lua> for LuaVectorIndex {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        if let Ok(LuaIndex(i)) = lua.unpack(value.clone()) {
            match u8::try_from(i) {
                Ok(i) => Ok(LuaVectorIndex(i)),
                Err(_) => Err(LuaError::FromLuaConversionError {
                    from: "number",
                    to: "vector index",
                    message: Some("out of range".to_string()),
                }),
            }
        } else if let LuaValue::String(s) = value {
            match s.to_str()?.parse() {
                Ok(LuaVectorIndex(i)) => Ok(LuaVectorIndex(i)),
                Err(e) => Err(LuaError::FromLuaConversionError {
                    from: "string",
                    to: "vector index",
                    message: Some(e),
                }),
            }
        } else {
            lua_convert_err(&value, "vector index (number or string)")
        }
    }
}

impl<'lua> IntoLua<'lua> for LuaVectorIndex {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
        self.0.checked_add(1).into_lua(&lua)
    }
}

impl FromStr for LuaVectorIndex {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.chars().exactly_one() {
            Ok(c) => match c {
                // Remember, Lua is 1-indexed so the X axis is 1.
                '1' | 'x' | 'X' => Ok(LuaVectorIndex(0)),
                '2' | 'y' | 'Y' => Ok(LuaVectorIndex(1)),
                '3' | 'z' | 'Z' => Ok(LuaVectorIndex(2)),
                '4' | 'w' | 'W' => Ok(LuaVectorIndex(3)),
                '5' | 'v' | 'V' => Ok(LuaVectorIndex(4)),
                '6' | 'u' | 'U' => Ok(LuaVectorIndex(5)),
                '7' | 't' | 'T' => Ok(LuaVectorIndex(6)),
                _ => Err(format!("no axis named '{c}'")),
            },
            Err(_) => Err("axis name must be single character".to_owned()),
        }
    }
}

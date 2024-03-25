use std::str::FromStr;

use itertools::Itertools;

use super::*;

/// Conversion wrapper for a string or integer specifying a component of a
/// vector.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct LuaVectorIndex(pub u8);
impl<'lua> FromLua<'lua> for LuaVectorIndex {
    fn from_lua(lua_value: LuaValue<'lua>, lua: LuaContext<'lua>) -> LuaResult<Self> {
        lua_convert!(match (lua, &lua_value, "vector index") {
            <_>(LuaIntegerNoConvert(i)) => {
                let i = u8::try_from(i).map_err(|_| LuaError::external("dimension count out of range"))?;
                Ok(LuaVectorIndex(i - 1))
            },
            LuaValue::String(s) => s.to_str()?.parse().map(|LuaVectorIndex(i)| LuaVectorIndex(i)),
        })
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
                '7' => Ok(LuaVectorIndex(6)),
                '8' => Ok(LuaVectorIndex(7)),
                '9' => Ok(LuaVectorIndex(8)),
                _ => Err(format!("no axis named '{c}'")),
            },
            Err(_) => Err("axis name must be single character".to_owned()),
        }
    }
}

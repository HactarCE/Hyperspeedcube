use hypermath::prelude::*;

use super::*;

/// Conversion wrapper for a number of dimensions.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct LuaNdim(pub u8);

impl<'lua> FromLua<'lua> for LuaNdim {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let LuaIntegerNoConvert(i) = lua.unpack(value)?;
        LuaNdim::try_from(i).map_err(|e| LuaError::FromLuaConversionError {
            from: "number",
            to: "number of dimensions",
            message: Some(e),
        })
    }
}

impl TryFrom<LuaInteger> for LuaNdim {
    type Error = String;

    fn try_from(value: LuaInteger) -> Result<Self, Self::Error> {
        if (1..=MAX_NDIM as _).contains(&value) {
            Ok(LuaNdim(value as u8))
        } else {
            Err("out of range".to_owned())
        }
    }
}

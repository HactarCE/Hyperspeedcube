use super::*;

/// Conversion wrapper for a Lua number that does not convert from a string.
pub struct LuaNumberNoConvert(pub LuaNumber);
impl<'lua> FromLua<'lua> for LuaNumberNoConvert {
    fn from_lua(value: LuaValue<'lua>, _lua: &'lua Lua) -> LuaResult<Self> {
        match value {
            LuaValue::Integer(x) => Ok(LuaNumberNoConvert(x as LuaNumber)),
            LuaValue::Number(x) => Ok(LuaNumberNoConvert(x)),
            _ => lua_convert_err(&value, "number"),
        }
    }
}

/// Conversion wrapper for a Lua integer that does not convert from a string,
/// but does convert from a floating point number with an exact integer value.
pub struct LuaIntegerNoConvert(pub LuaInteger);
impl<'lua> FromLua<'lua> for LuaIntegerNoConvert {
    fn from_lua(value: LuaValue<'lua>, _lua: &'lua Lua) -> LuaResult<Self> {
        match value {
            LuaValue::Integer(x) => Ok(LuaIntegerNoConvert(x)),
            LuaValue::Number(x) if x % 1.0 == 0.0 => Ok(LuaIntegerNoConvert(x as LuaInteger)),
            _ => lua_convert_err(&value, "integer"),
        }
    }
}

/// Same as `LuaIntegerNoConvert` but subtracts 1 to account for the different
/// between 0-indexing in Rust and 1-indexing in Lua.
pub struct LuaIndex(pub usize);
impl<'lua> FromLua<'lua> for LuaIndex {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        // IIFE to mimic try_block
        let index = (|| {
            let LuaIntegerNoConvert(i) = lua.unpack(value.clone()).ok()?;
            usize::try_from(i).ok()?.checked_sub(1)
        })();
        match index {
            Some(i) => Ok(LuaIndex(i)),
            None => lua_convert_err(&value, "positive integer"),
        }
    }
}
impl<'lua> IntoLua<'lua> for LuaIndex {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
        match self.0.checked_add(1) {
            Some(i) => i.into_lua(lua),
            None => Err(LuaError::external("overflow")),
        }
    }
}

/// Same as `LuaIndex` but converts to `u8` instead of `usize`.
pub struct LuaMirrorIndex(pub u8);
impl<'lua> FromLua<'lua> for LuaMirrorIndex {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        // IIFE to mimic try_block
        let index = (|| {
            let LuaIntegerNoConvert(i) = lua.unpack(value.clone()).ok()?;
            u8::try_from(i).ok()?.checked_sub(1)
        })();
        match index {
            Some(i) => Ok(LuaMirrorIndex(i)),
            None => lua_convert_err(&value, "smallish positive integer"),
        }
    }
}

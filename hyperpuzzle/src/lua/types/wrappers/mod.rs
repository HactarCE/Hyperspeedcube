//! Small conversion wrappers for various Lua types (mostly numbers or strings).
//!
//! This is a wrapper type that just describes how a Lua value is converted to a
//! Rust value, along with the error messages that should be generated.

use rlua::prelude::*;

#[macro_use]
mod macros;
mod multivector_index;
mod ndim;
mod vector_index;

pub use macros::{LuaNamedUserData, LuaUserDataConvertWrap};
pub use multivector_index::{LuaMultivectorIndex, NiNo};
pub use ndim::LuaNdim;
pub use vector_index::LuaVectorIndex;

/// Conversion wrapper for a Lua number that does not convert from a string.
pub struct LuaNumberNoConvert(pub LuaNumber);
impl<'lua> FromLua<'lua> for LuaNumberNoConvert {
    fn from_lua(lua_value: LuaValue<'lua>, lua: LuaContext<'lua>) -> LuaResult<Self> {
        lua_convert!(match (lua, &lua_value, "number") {
            LuaValue::Integer(x) => Ok(LuaNumberNoConvert(x as LuaNumber)),
            LuaValue::Number(x) => Ok(LuaNumberNoConvert(x)),
        })
    }
}

/// Conversion wrapper for a Lua integer that does not convert from a string,
/// but does convert from a floating point number with an exact integer value.
pub struct LuaIntegerNoConvert(pub LuaInteger);
impl<'lua> FromLua<'lua> for LuaIntegerNoConvert {
    fn from_lua(lua_value: LuaValue<'lua>, lua: LuaContext<'lua>) -> LuaResult<Self> {
        lua_convert!(match (lua, &lua_value, "integer") {
            LuaValue::Integer(x) => Ok(LuaIntegerNoConvert(x)),
            LuaValue::Number(x) => {
                if x % 1.0 == 0.0 {
                    Ok(LuaIntegerNoConvert(x as LuaInteger))
                } else {
                    Err(String::new())
                }
            }
        })
    }
}

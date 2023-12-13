use hypermath::prelude::*;

use super::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct LuaNdim(pub u8);

impl<'lua> FromLua<'lua> for LuaNdim {
    fn from_lua(lua_value: LuaValue<'lua>, lua: LuaContext<'lua>) -> LuaResult<Self> {
        if matches!(lua_value, LuaValue::Nil) {
            return LuaNdim::get_global(lua).map(LuaNdim);
        }
        lua_convert!(match (lua, &lua_value, "number of dimensions") {
            <u8>(i) => if (1..=MAX_NDIM).contains(&i) {
                Ok(LuaNdim(i))
            } else {
                Err("out of range".to_owned())
            },
        })
    }
}

impl LuaNdim {
    pub fn get_global(lua: LuaContext<'_>) -> LuaResult<u8> {
        match lua.globals().get("NDIM")? {
            LuaNil => Err(LuaError::external(
                "unknown number of dimensions; set global \
                 `NDIM` variable or pass NDIM as argument",
            )),
            other_value => Ok(LuaNdim::from_lua(other_value, lua)?.0),
        }
    }
}

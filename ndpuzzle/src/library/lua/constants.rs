use rlua::prelude::*;

use super::util;

pub fn lua_construct_axes_table(lua: LuaContext<'_>) -> LuaResult<LuaTable> {
    let ret = lua.create_table()?;
    for (i, c) in crate::AXIS_NAMES.chars().enumerate() {
        ret.set(i, c.to_string())?;
        ret.set(c.to_string(), i)?;
        ret.set(c.to_ascii_lowercase().to_string(), i)?;
    }
    util::seal_table(lua, ret)
}

use hypermath::prelude::*;
use rlua::prelude::*;

pub fn lua_construct_axes_table(lua: LuaContext<'_>) -> LuaResult<LuaTable<'_>> {
    let ret = lua.create_table()?;
    for (i, c) in AXIS_NAMES.chars().enumerate() {
        ret.set(i, c.to_string())?;
        ret.set(c.to_string(), i)?;
        ret.set(c.to_ascii_lowercase().to_string(), i)?;
    }
    let read_only_metatable: LuaTable<'_> = lua.globals().get("READ_ONLY_METATABLE")?;
    ret.set_metatable(Some(read_only_metatable));
    Ok(ret)
}

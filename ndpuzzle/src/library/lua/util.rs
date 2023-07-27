use super::*;

pub fn seal_table<'lua>(lua: LuaContext<'lua>, t: LuaTable<'lua>) -> LuaResult<LuaTable<'lua>> {
    let seal_table: LuaFunction = lua.globals().get("seal_table")?;
    seal_table.call(t)
}

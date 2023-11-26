use super::*;
use crate::PuzzleDefinition;

impl<'lua> FromLua<'lua> for PuzzleDefinition {
    fn from_lua(lua_value: LuaValue<'lua>, lua: LuaContext<'lua>) -> LuaResult<Self> {
        let t = LuaTable::from_lua(lua_value, lua)?;

        let id = t.get("id")?;
        let name = t.get("name")?;
        let ndim = t
            .get("ndim")
            .map_err(|e| LuaError::external(format!("{id:?} has bad `ndim`: {e}")))?;

        Ok(PuzzleDefinition { id, name, ndim })
    }
}

use itertools::Itertools;

use super::*;

/// Conversion wrapper for a [`LuaTable`] that requires that the table only
/// contain sequential values.
#[derive(Debug, Default, Clone)]
pub struct LuaSequence<T>(pub Vec<T>);
impl<'lua, T: FromLua<'lua>> FromLua<'lua> for LuaSequence<T> {
    fn from_lua(value: LuaValue<'lua>, _lua: &'lua Lua) -> LuaResult<Self> {
        let LuaValue::Table(t) = value else {
            return lua_convert_err(&value, "table");
        };
        if t.raw_len() != t.clone().pairs::<LuaValue<'_>, LuaValue<'_>>().count() {
            return Err(LuaError::FromLuaConversionError {
                from: "table",
                to: "sequence",
                message: Some("values must be in sequence".to_string()),
            });
        };
        t.sequence_values().try_collect().map(Self)
    }
}

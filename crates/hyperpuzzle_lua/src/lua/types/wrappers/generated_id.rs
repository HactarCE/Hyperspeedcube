use super::*;

/// Conversion wrapper for an object ID represented using a string or table of
/// strings.
pub struct LuaGeneratedId(pub String);
impl FromLua for LuaGeneratedId {
    fn from_lua(value: LuaValue, lua: &Lua) -> LuaResult<Self> {
        let LuaVecString(strings) = lua.unpack(value)?;
        let Some(id) = strings.get(0) else {
            return Err(LuaError::external("expected ID; got empty table"));
        };
        Ok(Self(hyperpuzzle_core::generated_id(id, &strings[1..])))
    }
}

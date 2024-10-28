use super::*;

/// Conversion wrapper for a string or table of strings.
pub struct LuaVecString(pub Vec<String>);
impl FromLua for LuaVecString {
    fn from_lua(value: LuaValue, lua: &Lua) -> LuaResult<Self> {
        Ok(Self(match &value {
            LuaNil => vec![],
            LuaValue::String(s) => vec![s.to_string_lossy()],
            LuaValue::Table(_) => LuaSequence::from_lua(value, lua)?.0,
            _ => {
                return Err(LuaError::external(
                    "expected nil, string, or table of strings",
                ));
            }
        }))
    }
}

use super::*;

/// Conversion wrapper for a string or table of strings.
pub struct LuaVecString(pub Vec<String>);
impl<'lua> FromLua<'lua> for LuaVecString {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        Ok(Self(match &value {
            LuaNil => vec![],
            LuaValue::String(s) => vec![s.to_string_lossy().into_owned()],
            LuaValue::Table(_) => LuaSequence::from_lua(value, lua)?.0,
            _ => {
                return Err(LuaError::external(
                    "expected nil, string, or table of strings",
                ));
            }
        }))
    }
}

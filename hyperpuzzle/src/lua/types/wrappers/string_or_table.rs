use super::*;

/// Conversion wrapper for a string or table.
pub type LuaNilStringOrTable<'lua> = NilStringOrTable<LuaTable<'lua>>;
impl<'lua> FromLua<'lua> for LuaNilStringOrTable<'lua> {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        match value {
            LuaNil => Ok(Self::Nil),
            LuaValue::String(s) => Ok(Self::String(s.to_string_lossy().to_string())),
            LuaValue::Table(t) => Ok(Self::Table(t)),
            v => lua_convert_err(&v, "string or table"),
        }
    }
}
impl<'lua> LuaNilStringOrTable<'lua> {
    pub fn to_lua_registry(self, lua: &Lua) -> LuaResult<NilStringOrRegisteredTable> {
        self.try_map_table(|t| lua.create_registry_value(t))
    }
}

/// Rust value is either
pub type NilStringOrRegisteredTable = NilStringOrTable<LuaRegistryKey>;
impl NilStringOrRegisteredTable {
    pub fn from_lua_registry<'lua>(&self, lua: &'lua Lua) -> LuaResult<LuaNilStringOrTable<'lua>> {
        self.try_map_table_ref(|t| lua.registry_value(t))
    }
}

/// Rust value that is either `Nil`, a string, or something else that generally
/// corresponds to a Lua table.
#[derive(Debug, Clone)]
pub enum NilStringOrTable<T> {
    Nil,
    String(String),
    Table(T),
}
impl<T> NilStringOrTable<T> {
    fn try_map_table<U>(self, f: impl FnOnce(T) -> LuaResult<U>) -> LuaResult<NilStringOrTable<U>> {
        match self {
            Self::Nil => Ok(NilStringOrTable::Nil),
            Self::String(s) => Ok(NilStringOrTable::String(s)),
            Self::Table(t) => Ok(NilStringOrTable::Table(f(t)?)),
        }
    }
    fn try_map_table_ref<U>(
        &self,
        f: impl FnOnce(&T) -> LuaResult<U>,
    ) -> LuaResult<NilStringOrTable<U>> {
        match self {
            Self::Nil => Ok(NilStringOrTable::Nil),
            Self::String(s) => Ok(NilStringOrTable::String(s.clone())),
            Self::Table(t) => Ok(NilStringOrTable::Table(f(t)?)),
        }
    }
}

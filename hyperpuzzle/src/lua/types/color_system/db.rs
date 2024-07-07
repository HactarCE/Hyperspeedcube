use super::*;
use crate::library::{LibraryDb, LibraryFile};

/// Lua handle to the library of all known color systems.
#[derive(Debug, Default, Copy, Clone)]
pub struct LuaColorSystemDb;
impl LuaUserData for LuaColorSystemDb {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field("type", LuaStaticStr("colorsystemdb"));
    }
    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::Len, |lua, Self, ()| {
            Ok(LibraryDb::get(lua)?.lock().color_systems.len())
        });

        methods.add_method("add", |lua, Self, pair| {
            let (id, params): (String, _) = pair;
            let id = crate::validate_id(id).into_lua_err()?;
            let color_system = super::from_lua_table(lua, Some(id.clone()), params)?;
            LibraryFile::get_current(lua)?.define_color_system(id, color_system)
        });
    }
}

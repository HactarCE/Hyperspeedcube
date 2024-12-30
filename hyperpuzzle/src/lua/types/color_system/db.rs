use std::sync::Arc;

use super::*;
use crate::library::LibraryDb;

/// Lua handle to the library of all known color systems.
#[derive(Debug, Default, Copy, Clone)]
pub struct LuaColorSystemDb;
impl LuaUserData for LuaColorSystemDb {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field("type", LuaStaticStr("colorsystemdb"));
    }
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::Len, |lua, Self, ()| {
            Ok(LibraryDb::get(lua).lock().color_systems.len())
        });

        methods.add_method("add", |lua, Self, spec| {
            let color_system = super::from_lua_table(lua, spec)?;
            LibraryDb::get(lua)
                .lock()
                .color_systems
                .insert(color_system.id.clone(), Arc::new(color_system));
            Ok(())
        });
    }
}

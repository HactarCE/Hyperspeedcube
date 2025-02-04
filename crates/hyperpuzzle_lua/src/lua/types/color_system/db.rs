use std::sync::Arc;

use super::*;
use crate::lua::lua_warn_fn;

/// Lua handle to the catalog of all known color systems.
#[derive(Debug, Default, Copy, Clone)]
pub struct LuaColorSystemDb;
impl LuaUserData for LuaColorSystemDb {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field("type", LuaStaticStr("colorsystemdb"));
    }
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("add", |lua, Self, spec| {
            let color_system_spec = super::from_lua_table(lua, spec)?;
            let (color_system, _) = color_system_spec
                .build(None, None, lua_warn_fn(lua))
                .map_err(|e| LuaError::external(format!("{e:#}")))?;
            crate::lua::LuaLoader::get_catalog(lua).add_color_system(Arc::new(color_system));
            Ok(())
        });
    }
}

use std::sync::Arc;

use super::*;

/// Lua handle to the catalog of all known color system generators.
#[derive(Debug, Default, Copy, Clone)]
pub struct LuaColorSystemGeneratorDb;
impl LuaUserData for LuaColorSystemGeneratorDb {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field("type", LuaStaticStr("colorsystemgeneratordb"));
    }
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("add", |lua, Self, spec: LuaColorSystemGeneratorSpec| {
            crate::lua::LuaLoader::get_catalog(lua)
                .add_color_system_generator(Arc::new(spec.into_color_system_generator(lua)));
            Ok(())
        });
    }
}

use std::sync::Arc;

use super::*;

/// Lua handle to the catalog of all known puzzle generators.
#[derive(Debug, Default, Copy, Clone)]
pub struct LuaPuzzleGeneratorDb;
impl LuaUserData for LuaPuzzleGeneratorDb {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field("type", LuaStaticStr("puzzlegeneratordb"));
    }
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("add", |lua, Self, spec: LuaPuzzleGeneratorSpec| {
            crate::lua::LuaLoader::get_catalog(lua)
                .add_puzzle_generator(Arc::new(spec.into_puzzle_spec_generator(lua)));
            Ok(())
        });
    }
}

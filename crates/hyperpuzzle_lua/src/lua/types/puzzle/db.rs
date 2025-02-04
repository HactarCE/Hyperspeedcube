use std::sync::Arc;

use super::*;

/// Lua handle to the catalog of all known puzzles.
#[derive(Debug, Default, Copy, Clone)]
pub struct LuaPuzzleDb;
impl LuaUserData for LuaPuzzleDb {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field("type", LuaStaticStr("puzzledb"));
    }
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("add", |lua, Self, spec: LuaPuzzleSpec| {
            crate::lua::LuaLoader::get_catalog(lua)
                .add_puzzle(Arc::new(spec.into_puzzle_spec(lua)));
            Ok(())
        });
    }
}

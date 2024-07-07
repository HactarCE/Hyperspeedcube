use super::*;
use crate::library::{LibraryDb, LibraryFile};

/// Lua handle to the library of all known puzzles.
#[derive(Debug, Default, Copy, Clone)]
pub struct LuaPuzzleDb;
impl LuaUserData for LuaPuzzleDb {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field("type", LuaStaticStr("puzzledb"));
    }
    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::Len, |lua, Self, ()| {
            Ok(LibraryDb::get(lua)?.lock().puzzles.len())
        });

        methods.add_method("add", |lua, Self, pair| {
            let (id, mut params): (String, PuzzleParams) = pair;
            params.id = crate::validate_id(id.clone()).into_lua_err()?;
            LibraryFile::get_current(lua)?.define_puzzle(id, params)
        });
    }
}

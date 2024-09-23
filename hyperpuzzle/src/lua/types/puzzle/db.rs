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

        methods.add_method("add", |lua, Self, params| {
            LibraryFile::get_current(lua)?.define_puzzle(params)
        });
    }
}

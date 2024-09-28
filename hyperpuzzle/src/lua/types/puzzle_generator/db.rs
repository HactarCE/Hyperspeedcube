use super::*;
use crate::library::{LibraryDb, LibraryFile};

/// Lua handle to the library of all known puzzles.
#[derive(Debug, Default, Copy, Clone)]
pub struct LuaPuzzleGeneratorDb;
impl LuaUserData for LuaPuzzleGeneratorDb {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field("type", LuaStaticStr("puzzlegeneratordb"));
    }
    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::Len, |lua, Self, ()| {
            Ok(LibraryDb::get(lua)?.lock().puzzle_generators.len())
        });

        methods.add_method("add", |lua, Self, params| {
            LibraryFile::get_current(lua)?.define_puzzle_generator(params)
        });
    }
}

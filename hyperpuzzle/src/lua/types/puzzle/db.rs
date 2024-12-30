use std::sync::Arc;

use super::*;
use crate::library::LibraryDb;

/// Lua handle to the library of all known puzzles.
#[derive(Debug, Default, Copy, Clone)]
pub struct LuaPuzzleDb;
impl LuaUserData for LuaPuzzleDb {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field("type", LuaStaticStr("puzzledb"));
    }
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::Len, |lua, Self, ()| {
            Ok(LibraryDb::get(lua).lock().puzzles.len())
        });

        methods.add_method("add", |lua, Self, spec| {
            let puzzle_spec = PuzzleSpec::from_lua(spec, lua)?;
            LibraryDb::get(lua)
                .lock()
                .puzzles
                .insert(puzzle_spec.id.clone(), Arc::new(puzzle_spec));
            Ok(())
        });
    }
}

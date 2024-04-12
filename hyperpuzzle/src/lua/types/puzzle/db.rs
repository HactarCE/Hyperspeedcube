use crate::library::{Cached, LibraryDb, LibraryFile};

use super::*;

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
            params.id = id.clone();
            LibraryFile::get_current(lua)?
                .as_loading()?
                .puzzles
                .insert(id, Cached::new(params));
            Ok(())
        });
    }
}

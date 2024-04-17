use super::*;
use crate::library::{LibraryDb, LibraryFile};

/// Lua handle to the library database of all known twist systems.
#[derive(Debug, Default, Copy, Clone)]
pub struct LuaTwistSystemDb;
impl LuaUserData for LuaTwistSystemDb {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field("type", LuaStaticStr("twistsystemdb"));
    }
    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::Len, |lua, Self, ()| {
            Ok(LibraryDb::get(lua)?.lock().twist_systems.len())
        });

        methods.add_method("add", |lua, Self, pair: (String, TwistSystemParams)| {
            let (id, mut params) = pair;
            params.id = Some(id.clone());
            LibraryFile::get_current(lua)?.define(id, params)
        })
    }
}

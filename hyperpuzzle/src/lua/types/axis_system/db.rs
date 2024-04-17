use super::*;
use crate::library::{Cached, LibraryDb, LibraryFile};

#[derive(Debug, Default, Copy, Clone)]
pub struct LuaAxisSystemDb;

impl LuaUserData for LuaAxisSystemDb {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field("type", LuaStaticStr("axissystemdb"));
    }
    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::Len, |lua, Self, ()| {
            Ok(LibraryDb::get(lua)?.lock().axis_systems.len())
        });

        methods.add_method("add", |lua, Self, pair| {
            let (id, mut params): (String, AxisSystemParams) = pair;
            params.id = Some(id.clone());
            LibraryFile::get_current(lua)?
                .as_loading()?
                .axis_systems
                .insert(id, Cached::new(params));
            Ok(())
        })
    }
}

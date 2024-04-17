use super::*;
use crate::library::{LibraryDb, LibraryFile};

#[derive(Debug, Default, Copy, Clone)]
pub struct LuaShapeDb;
impl LuaUserData for LuaShapeDb {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field("type", LuaStaticStr("shapedb"));
    }
    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::Len, |lua, Self, ()| {
            Ok(LibraryDb::get(lua)?.lock().shapes.len())
        });

        methods.add_method("add", |lua, Self, pair| {
            let (id, mut params): (String, ShapeParams) = pair;
            params.id = Some(id.clone());
            LibraryFile::get_current(lua)?.insert(id, params)
        });
    }
}

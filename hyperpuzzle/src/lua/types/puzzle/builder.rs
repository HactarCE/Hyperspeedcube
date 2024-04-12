use std::sync::Arc;

use parking_lot::{Mutex, MutexGuard};

use crate::builder::PuzzleBuilder;

use super::*;

#[derive(Debug, Clone)]
pub struct LuaPuzzleBuilder(pub Arc<Mutex<PuzzleBuilder>>);

impl<'lua> FromLua<'lua> for LuaPuzzleBuilder {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        cast_userdata(lua, &value)
    }
}

impl LuaPuzzleBuilder {
    fn lock(&self) -> MutexGuard<'_, PuzzleBuilder> {
        self.0.lock()
    }
}

impl LuaUserData for LuaPuzzleBuilder {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("puzzle"));

        fields.add_field_method_get("id", |_lua, this| Ok(this.lock().id.clone()));
        fields.add_field_method_get("space", |_lua, this| Ok(LuaSpace(this.lock().space())));
        fields.add_field_method_get("ndim", |_lua, this| Ok(this.lock().ndim()));

        fields.add_field_method_get("shape", |_lua, this| {
            Ok(LuaShape(Arc::clone(&this.lock().shape)))
        });
        fields.add_field_method_get("twists", |_lua, this| {
            Ok(LuaTwistSystem(Arc::clone(&this.lock().twists)))
        });
        fields.add_field_method_get("axes", |_lua, this| {
            Ok(LuaAxisSystem(Arc::clone(&this.lock().twists.lock().axes)))
        });
    }

    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {}
}

use std::sync::Arc;

use parking_lot::{Mutex, MutexGuard};

use super::*;
use crate::builder::PuzzleBuilder;

/// Lua handle to a puzzle under construction.
#[derive(Debug, Clone)]
pub struct LuaPuzzleBuilder(pub Arc<Mutex<PuzzleBuilder>>);

impl<'lua> FromLua<'lua> for LuaPuzzleBuilder {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        cast_userdata(lua, &value)
    }
}

impl LuaUserData for LuaPuzzleBuilder {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("puzzle"));

        fields.add_field_method_get("id", |_lua, this| Ok(this.lock().id.clone()));
        fields.add_field_method_get("space", |_lua, this| Ok(LuaSpace(this.lock().space())));
        fields.add_field_method_get("ndim", |_lua, this| Ok(this.lock().ndim()));

        fields.add_field_method_get("shape", |_lua, this| Ok(LuaShape(this.arc())));
        fields.add_field_method_get("colors", |_lua, this| Ok(LuaColorSystem(this.arc())));
        fields.add_field_method_get("twists", |_lua, this| Ok(LuaTwistSystem(this.arc())));
        fields.add_field_method_get("axes", |_lua, this| Ok(LuaAxisSystem(this.arc())));
    }

    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, |_lua, this, ()| {
            Ok(format!("puzzle({:?})", this.lock().name))
        });

        // Shortcut methods (could be called on `.shape` instead)
        methods.add_method("carve", |lua, this, cuts| {
            LuaShape(this.arc()).cut(lua, cuts, CutMode::Carve, StickerMode::NewColor)
        });
        methods.add_method("carve_unstickered", |lua, this, cuts| {
            LuaShape(this.arc()).cut(lua, cuts, CutMode::Carve, StickerMode::None)
        });
        methods.add_method("slice", |lua, this, cuts| {
            LuaShape(this.arc()).cut(lua, cuts, CutMode::Slice, StickerMode::None)
        });

        // Shortcut methods (could be called on `.axes` instead)
        methods.add_method("add_axes", |lua, this, (vectors, extra)| {
            let ret = LuaAxisSystem(this.arc()).add(lua, vectors, extra)?;

            for axis in &ret {
                for cut in axis.layers().cuts()? {
                    let mut puz = this.lock();
                    puz.shape.slice(None, cut, None, None).into_lua_err()?;
                }
            }

            Ok(ret)
        });
        methods.add_method("add_axes_unsliced", |lua, this, (vectors, extra)| {
            LuaAxisSystem(this.arc()).add(lua, vectors, extra)
        });
    }
}

impl LuaPuzzleBuilder {
    /// Returns a mutex guard granting temporary access to the underlying
    /// [`PuzzleBuilder`].
    pub fn lock(&self) -> MutexGuard<'_, PuzzleBuilder> {
        self.0.lock()
    }

    /// Returns a reference to the underlying puzzle builder.
    pub fn arc(&self) -> Arc<Mutex<PuzzleBuilder>> {
        Arc::clone(&self.0)
    }
}

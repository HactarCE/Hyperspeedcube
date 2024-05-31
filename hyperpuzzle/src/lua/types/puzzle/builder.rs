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

        fields.add_field_method_get("shape", |_lua, this| {
            Ok(LuaShape(Arc::clone(&this.lock().shape)))
        });
        fields.add_field_method_get("colors", |_lua, this| {
            Ok(LuaColorSystem(Arc::clone(&this.lock().shape)))
        });
        fields.add_field_method_get("twists", |_lua, this| {
            Ok(LuaTwistSystem(Arc::clone(&this.lock().twists)))
        });
        fields.add_field_method_get("axes", |_lua, this| {
            Ok(LuaAxisSystem(Arc::clone(&this.lock().twists.lock().axes)))
        });
    }

    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, |_lua, this, ()| {
            Ok(format!("puzzle({:?})", this.lock().name))
        });

        // Shortcut methods (could be called on `.shape` instead)
        methods.add_method("carve", |lua, this, cuts| {
            this.shape()
                .cut(lua, cuts, CutMode::Carve, StickerMode::NewColor)
        });
        methods.add_method("carve_unstickered", |lua, this, cuts| {
            this.shape()
                .cut(lua, cuts, CutMode::Carve, StickerMode::None)
        });
        methods.add_method("slice", |lua, this, cuts| {
            this.shape()
                .cut(lua, cuts, CutMode::Slice, StickerMode::None)
        });

        // Shortcut methods (could be called on `.axes` instead)
        methods.add_method("add_axes", |lua, this, (vectors, extra)| {
            let ret = LuaAxisSystem(this.lock().axis_system()).add(lua, vectors, extra)?;

            let shape = this.shape();
            let mut shape_guard = shape.lock();
            for axis in &ret {
                for cut in axis.layers().cuts()? {
                    shape_guard.slice(None, cut, None, None).into_lua_err()?;
                }
            }

            Ok(ret)
        });
        methods.add_method("add_axes_unsliced", |lua, this, (vectors, extra)| {
            LuaAxisSystem(this.lock().axis_system()).add(lua, vectors, extra)
        });
    }
}

impl LuaPuzzleBuilder {
    /// Returns a mutex guard granting temporary access to the underlying
    /// [`PuzzleBuilder`].
    pub fn lock(&self) -> MutexGuard<'_, PuzzleBuilder> {
        self.0.lock()
    }

    /// Returns the shape of the puzzle.
    pub fn shape(&self) -> LuaShape {
        LuaShape(Arc::clone(&self.lock().shape))
    }
}

use std::sync::Arc;

use parking_lot::{Mutex, MutexGuard};

use super::*;
use crate::builder::PuzzleBuilder;
use crate::lua::lua_warn_fn;
use crate::Color;

/// Lua handle to a shape under construction.
#[derive(Debug, Clone)]
pub struct LuaShape(pub Arc<Mutex<PuzzleBuilder>>);

impl<'lua> FromLua<'lua> for LuaShape {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        cast_userdata(lua, &value)
    }
}

impl LuaUserData for LuaShape {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("shape"));

        fields.add_field_method_get("ndim", |_lua, this| Ok(this.lock().ndim()));
    }

    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, |_lua, this, ()| {
            let this = this.lock();
            let ndim = this.ndim();
            Ok(format!("shape(ndim={ndim})"))
        });

        methods.add_method("carve", |lua, this, cuts| {
            this.cut(lua, cuts, CutMode::Carve, StickerMode::NewColor)
        });
        methods.add_method("carve_unstickered", |lua, this, cuts| {
            this.cut(lua, cuts, CutMode::Carve, StickerMode::None)
        });
        methods.add_method("slice", |lua, this, cuts| {
            this.cut(lua, cuts, CutMode::Slice, StickerMode::None)
        });
    }
}

impl LuaShape {
    /// Returns a mutex guard granting temporary access to the underlying
    /// [`PuzzleBuilder`].
    pub fn lock(&self) -> MutexGuard<'_, PuzzleBuilder> {
        self.0.lock()
    }

    /// Cut the puzzle.
    pub fn cut<'lua>(
        &self,
        lua: &'lua Lua,
        cuts: LuaSymmetricSet<LuaHyperplane>,
        cut_mode: CutMode,
        sticker_mode: StickerMode,
    ) -> LuaResult<()> {
        let mut puz = self.lock();
        let shape = &mut puz.shape;

        for (name, LuaHyperplane(plane)) in cuts.to_vec(lua)? {
            let color = match sticker_mode {
                StickerMode::NewColor => Some({
                    let c = shape.colors.add(vec![plane.clone()]).into_lua_err()?;
                    shape.colors.names.set(c, name, lua_warn_fn(lua));
                    c
                }),
                StickerMode::None => None,
                StickerMode::Color(c) => Some(c),
            };
            match cut_mode {
                CutMode::Carve => shape.carve(None, plane, color).into_lua_err()?,
                CutMode::Slice => shape.slice(None, plane, color, color).into_lua_err()?,
            }
        }

        Ok(())
    }
}

/// Which pieces to keep when cutting the shape.
pub enum CutMode {
    /// Delete any pieces outside the cut; keep only pieces inside the cut.
    Carve,
    /// Keep all pieces on both sides of the cut.
    Slice,
}
/// How to sticker new facets created by a cut.
pub enum StickerMode {
    /// Add a new color for each cut and create new stickers with that color on
    /// both sides of the cut.
    NewColor,
    /// Do not add new stickers.
    None,
    /// Add new stickers, all with the same existing color.
    Color(Color),
}

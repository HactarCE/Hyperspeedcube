use std::sync::Arc;

use parking_lot::{Mutex, MutexGuard};

use super::*;
use crate::builder::PuzzleBuilder;
use crate::lua::lua_warn_fn;
use crate::Color;

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

        fields.add_field_method_get("colors", |_lua, this| Ok(LuaColorSystem(this.arc())));
        fields.add_field_method_get("axes", |_lua, this| Ok(LuaAxisSystem(this.arc())));
        fields.add_field_method_get("twists", |_lua, this| Ok(LuaTwistSystem(this.arc())));
    }

    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, |_lua, this, ()| {
            Ok(format!("puzzle({:?})", this.lock().name))
        });

        methods.add_method("carve", |lua, this, (cuts, args)| {
            let sticker_mode;
            if let Some(table) = args {
                let stickers: LuaValue<'_>;
                unpack_table!(lua.unpack(table { stickers }));
                if stickers.is_nil() {
                    sticker_mode = StickerMode::NewColor; // default
                } else if let Some(stickers) = stickers.as_boolean() {
                    match stickers {
                        true => sticker_mode = StickerMode::NewColor,
                        false => sticker_mode = StickerMode::None,
                    }
                } else if let Ok(color) = LuaColor::from_lua(stickers.clone(), lua) {
                    sticker_mode = StickerMode::Color(color.id);
                } else {
                    return Err(LuaError::external("bad value for key \"stickers\""));
                }
            } else {
                sticker_mode = StickerMode::NewColor;
            }
            // TODO: allow naming a single carve operation
            this.cut(lua, cuts, CutMode::Carve, sticker_mode)
        });
        methods.add_method("slice", |lua, this, cuts| {
            this.cut(lua, cuts, CutMode::Slice, StickerMode::None)
        });

        methods.add_method("add_axes", |lua, this, (vectors, extra)| {
            let (new_axes, slice) = LuaAxisSystem(this.arc()).add(lua, vectors, extra)?;

            if slice == SliceAxisLayers::Slice {
                for axis in &new_axes {
                    for cut in axis.layers().cuts()? {
                        let mut puz = this.lock();
                        puz.shape.slice(None, cut, None, None).into_lua_err()?;
                    }
                }
            }

            Ok(new_axes)
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

    /// Cut the puzzle.
    fn cut<'lua>(
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
enum CutMode {
    /// Delete any pieces outside the cut; keep only pieces inside the cut.
    Carve,
    /// Keep all pieces on both sides of the cut.
    Slice,
}

/// How to sticker new facets created by a cut.
enum StickerMode {
    /// Add a new color for each cut and create new stickers with that color on
    /// both sides of the cut.
    NewColor,
    /// Do not add new stickers.
    None,
    /// Add new stickers, all with the same existing color.
    Color(Color),
}

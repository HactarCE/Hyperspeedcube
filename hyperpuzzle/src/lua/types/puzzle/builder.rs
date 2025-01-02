use std::collections::HashMap;
use std::sync::Arc;

use eyre::Context;
use itertools::Itertools;
use parking_lot::{Mutex, MutexGuard};

use super::*;
use crate::builder::PuzzleBuilder;
use crate::lua::lua_warn_fn;
use crate::{Color, DevOrbit};

/// Lua handle to a puzzle under construction.
#[derive(Debug, Clone)]
pub struct LuaPuzzleBuilder(pub Arc<Mutex<PuzzleBuilder>>);

impl FromLua for LuaPuzzleBuilder {
    fn from_lua(value: LuaValue, lua: &Lua) -> LuaResult<Self> {
        cast_userdata(lua, &value)
    }
}

impl LuaUserData for LuaPuzzleBuilder {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("puzzle"));

        fields.add_field_method_get("id", |_lua, this| Ok(this.lock().id.clone()));
        fields.add_field_method_get("space", |_lua, this| Ok(LuaSpace(this.lock().space())));
        fields.add_field_method_get("ndim", |_lua, this| Ok(this.lock().ndim()));

        fields.add_field_method_get("colors", |_lua, this| Ok(LuaColorSystem(this.arc())));
        fields.add_field_method_get("axes", |_lua, this| Ok(LuaAxisSystem(this.arc())));
        fields.add_field_method_get("twists", |_lua, this| Ok(LuaTwistSystem(this.arc())));
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, |_lua, this, ()| {
            Ok(format!("puzzle({:?})", this.lock().name))
        });

        methods.add_method("carve", |lua, this, (cuts, args)| {
            let sticker_mode;
            if let Some(table) = args {
                let stickers: LuaValue;
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
                } else if let LuaValue::Table(mapping_table) = stickers {
                    let mut puz = this.lock();
                    let color_system = &mut puz.shape.colors;
                    sticker_mode = StickerMode::Map(
                        mapping_table
                            .pairs()
                            .map(|pair| {
                                let (name, LuaNameSet(color_name)) = pair?;
                                let color = color_system
                                    .get_or_add_with_name(color_name, lua_warn_fn(lua))?;
                                eyre::Ok((name, color))
                            })
                            .try_collect()
                            .wrap_err("error constructing color mapping")
                            .map_err(|e| LuaError::external(format!("{e:#}")))?,
                    );
                } else {
                    return Err(LuaError::external("bad value for key \"stickers\""));
                }
            } else {
                sticker_mode = StickerMode::NewColor;
            }
            // TODO: allow assigning face name when carving a single face (i.e., not using orbits)
            this.cut(lua, cuts, CutMode::Carve, sticker_mode)
        });
        methods.add_method("slice", |lua, this, cuts| {
            this.cut(lua, cuts, CutMode::Slice, StickerMode::None)
        });

        methods.add_method("add_piece_type", |lua, this, args| {
            let name: String;
            let display: Option<String>;
            (name, display) = args;

            if let Err(e) = this.lock().shape.get_or_add_piece_type(name, display) {
                lua.warning(e.to_string(), false);
            }
            Ok(())
        });
        methods.add_method("mark_piece", |lua, this, args| {
            let region: LuaRegion;
            let name: String;
            let display: Option<String>;
            (region, name, display) = args;

            this.lock()
                .shape
                .mark_piece_by_region(
                    &name,
                    display,
                    |point| region.contains_point(point),
                    lua_warn_fn(lua),
                )
                .into_lua_err()
        });
        methods.add_method("unify_piece_types", |lua, this, sym: LuaSymmetry| {
            let transforms = sym.chiral_safe_generators();
            this.lock()
                .shape
                .unify_piece_types(&transforms, lua_warn_fn(lua));
            Ok(())
        });
        methods.add_method("delete_untyped_pieces", |lua, this, ()| {
            this.lock().shape.delete_untyped_pieces(lua_warn_fn(lua));
            Ok(())
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
    fn cut(
        &self,
        lua: &Lua,
        cuts: LuaSymmetricSet<LuaHyperplane>,
        cut_mode: CutMode,
        sticker_mode: StickerMode,
    ) -> LuaResult<()> {
        let mut puz = self.lock();
        let shape = &mut puz.shape;
        let mut gen_seqs = vec![];
        let mut colors_assigned = vec![];
        for (gen_seq, name, LuaHyperplane(plane)) in cuts.to_vec(lua)? {
            gen_seqs.push(gen_seq);

            let color = match &sticker_mode {
                StickerMode::NewColor => Some({
                    let name_string = name.as_ref().and_then(|name| name.canonical_name());
                    match name_string.and_then(|s| shape.colors.names.names_to_ids().get(&s)) {
                        // Use an existing color unmodified.
                        Some(&existing_color) => existing_color,
                        // Create new color.
                        None => {
                            let c = shape.colors.add().into_lua_err()?;
                            shape.colors.names.set_name(c, name, lua_warn_fn(lua));
                            c
                        }
                    }
                }),
                StickerMode::None => None,
                StickerMode::Color(c) => Some(*c),
                StickerMode::Map(m) => name.as_ref().and_then(|name| match name.string_set() {
                    Ok(strings) => strings.iter().find_map(|s| m.get(s)).copied(),
                    Err(e) => {
                        lua.warning(e.to_string(), false);
                        None
                    }
                }),
            };
            colors_assigned.push(color);

            match cut_mode {
                CutMode::Carve => shape.carve(None, plane, color),
                CutMode::Slice => shape.slice(None, plane, color, color),
            }
            .map_err(|e| LuaError::external(format!("{e:#}")))?;
        }

        shape.colors.color_orbits.push(DevOrbit {
            kind: "colors",
            elements: colors_assigned,
            generator_sequences: gen_seqs,
        });

        Ok(())
    }
}

/// Which pieces to keep when cutting the shape.
#[derive(Debug)]
enum CutMode {
    /// Delete any pieces outside the cut; keep only pieces inside the cut.
    Carve,
    /// Keep all pieces on both sides of the cut.
    Slice,
}

/// How to sticker new facets created by a cut.
#[derive(Debug, Default)]
enum StickerMode {
    /// Add a new color for each cut and create new stickers with that color on
    /// both sides of the cut.
    #[default]
    NewColor,
    /// Do not add new stickers.
    None,
    /// Add new stickers, all with the same existing color.
    Color(Color),
    /// Adds new colors or uses existing colors as determined by the mapping.
    Map(HashMap<String, Color>),
}

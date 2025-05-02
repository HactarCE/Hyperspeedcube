use std::collections::HashMap;
use std::sync::Arc;

use hypermath::Hyperplane;

use crate::package::types::{name_strategy::RhaiNameStrategy, symmetry::RhaiSymmetry};

use super::*;

impl RhaiPuzzle {
    pub fn unpack_cut_args(
        &self,
        ctx: &Ctx<'_>,
        args: Option<Map>,
        mode: CutMode,
        default_stickers: StickerMode,
    ) -> Result<CutArgs> {
        let Some(args_map) = args else {
            return Ok(CutArgs {
                mode,
                stickers: default_stickers,
                // region: None, // TODO
            });
        };

        let_from_map!(ctx, args_map, {
            let stickers: Option<Dynamic>;
            // let region: Option<Region>; // TODO
        });

        let stickers = stickers.unwrap_or(Dynamic::TRUE);

        let sticker_mode;
        if let Ok(stickers) = stickers.as_bool() {
            match stickers {
                true => sticker_mode = default_stickers,
                false => sticker_mode = StickerMode::None,
            }
        } else if let Ok(color) = from_rhai::<RhaiColor>(ctx, stickers.clone()) {
            // TODO: avoid `stickers.clone()`
            // TODO: support color name
            sticker_mode = StickerMode::Color(color.id);
        } else if let Ok(map) = from_rhai::<Map>(ctx, stickers.clone()) {
            // TODO: avoid `stickers.clone()`
            todo!()

            // let mut puz = self.lock();
            // let color_system = &mut puz.shape.colors;
            // sticker_mode = StickerMode::Map(
            //     map.pairs()
            //         .map(|pair| {
            //             let (name, LuaNameSet(color_name)) = pair?;
            //             let color =
            //                 color_system.get_or_add_with_name(color_name, lua_warn_fn(lua))?;
            //             eyre::Ok((name, color))
            //         })
            //         .try_collect()
            //         .wrap_err("error constructing color mapping")
            //         .map_err(|e| LuaError::external(format!("{e:#}")))?,
            // );
        } else {
            Err(ConvertError::new_expected_str(ctx, todo!(), Some(&stickers)).in_key("stickers"))?;
        }

        Ok(CutArgs {
            mode,
            stickers: sticker_mode,
            // region: None, // TODO
        })
    }

    /// Cut the puzzle.
    pub fn cut(
        &self,
        ctx: &Ctx<'_>,
        plane: Hyperplane,
        names: RhaiNameStrategy,
        cut_mode: CutMode,
        args: Option<Map>,
        default_sticker_mode: StickerMode,
    ) -> Result<()> {
        let CutArgs {
            mode,
            stickers,
            // region,
        } = self.unpack_cut_args(ctx, args, cut_mode, default_sticker_mode)?;

        let mut puz = self.lock()?;
        let shape = &mut puz.shape;
        let mut gen_seqs = vec![];
        let mut colors_assigned = vec![];
        match RhaiSymmetry::get(ctx) {
            Some(sym) => {
                for (gen_seq, motor, cut, name) in
                    sym.orbit_with_names::<Color, _>(ctx, plane, &names)?
                {
                    gen_seqs.push(gen_seq);

                    let color = match &stickers {
                        StickerMode::NewColor => Some(match name {
                            Some(name) => shape
                                .colors
                                .get_or_add_with_name(name, void_warn(ctx))
                                .eyrefmt()?,
                            None => shape.colors.add().eyrefmt()?,
                        }),
                        StickerMode::None => None,
                        StickerMode::Color(c) => Some(*c),
                        // TODO: what if `name` is a name spec?
                        StickerMode::Map(m) => name.and_then(|name| m.get(&name).copied()),
                    };
                    colors_assigned.push(color);

                    // TODO
                    let piece_set = None;
                    // let piece_set = region.as_ref().map(|r| {
                    //     shape
                    //         .active_pieces_in_region(|point| r.contains_point(point))
                    //         .collect()
                    // });

                    match mode {
                        CutMode::Carve => shape.carve(piece_set.as_ref(), cut, color),
                        CutMode::Slice => shape.slice(piece_set.as_ref(), cut, color),
                    }
                    .eyrefmt()?;
                }

                shape.colors.orbits.push(Orbit {
                    elements: Arc::new(colors_assigned),
                    generator_sequences: Arc::new(gen_seqs),
                });
            }
            None => {}
        }

        Ok(())
    }
}

/// Cut arguments.
#[derive(Debug)]
pub struct CutArgs {
    mode: CutMode,
    stickers: StickerMode,
    // region: Option<LuaRegion>, // TODO
}

/// Which pieces to keep when cutting the shape.
#[derive(Debug)]
pub enum CutMode {
    /// Delete any pieces outside the cut; keep only pieces inside the cut.
    Carve,
    /// Keep all pieces on both sides of the cut.
    Slice,
}

/// How to sticker new facets created by a cut.
#[derive(Debug, Default)]
pub enum StickerMode {
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

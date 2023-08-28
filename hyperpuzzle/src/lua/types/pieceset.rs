use tinyset::Set64;

use super::*;
use crate::{Color, PieceBuilder, PieceSet, StickerBuilder};

lua_userdata_value_conversion_wrapper! {
    #[name = "pieceset"]
    pub struct LuaPieceSet(PieceSet) ;
}

impl LuaUserData for LuaNamedUserData<PieceSet> {
    fn add_methods<'lua, T: LuaUserDataMethods<'lua, Self>>(methods: &mut T) {
        methods.add_method_mut("carve", |lua, Self(this), LuaManifold(m)| {
            LuaSpace::with(lua, |space| {
                LuaPuzzleBuilder::with(lua, |puzzle| {
                    let sticker_color =
                        puzzle.colors.push(()).map_err(|e| LuaError::external(e))?;

                    let mut cutter = space.carve(m);
                    let mut new_pieces = Set64::new();
                    for piece in this.0.iter() {
                        let old_piece = &mut puzzle.pieces[piece];
                        let result = cutter
                            .cut(old_piece.shape)
                            .map_err(|e| LuaError::external(e.context("cutting piece")))?;
                        if let Some(shape) = result.inside {
                            let mut new_piece = PieceBuilder {
                                shape,
                                stickers: vec![],
                                is_active: old_piece.is_active,
                            };
                            for old_sticker in &old_piece.stickers {
                                let result = cutter.cut(old_sticker.shape).map_err(|e| {
                                    LuaError::external(e.context("cutting sticker"))
                                })?;
                                if let Some(sticker_shape) = result.inside {
                                    new_piece.stickers.push(StickerBuilder {
                                        shape: sticker_shape,
                                        color: old_sticker.color,
                                    });
                                }
                            }
                            if let Some(sticker_shape) = result.flush_facet {
                                new_piece.stickers.push(StickerBuilder {
                                    shape: sticker_shape,
                                    color: sticker_color,
                                });
                            }
                            let new_piece_id = puzzle
                                .pieces
                                .push(new_piece)
                                .map_err(|e| LuaError::external(e))?;
                            new_pieces.insert(new_piece_id);
                        }
                        puzzle.pieces[piece].is_active = false;
                    }

                    *this = PieceSet(new_pieces);
                    Ok(())
                })
            })
        });
    }
}

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Weak};

use anyhow::{Context, Result};
use hypermath::prelude::*;
use hypershape::prelude::*;
use itertools::Itertools;
use parking_lot::Mutex;
use smallvec::smallvec;

use super::simplices::{Simplexifier, VertexId};
use super::{
    Color, MeshBuilder, NotationScheme, PerColor, PerPiece, PerSticker, Piece, PieceInfo, PieceSet,
    PieceType, PieceTypeInfo, Puzzle, PuzzleState, StickerInfo,
};

#[derive(Debug)]
pub struct PuzzleBuilder {
    pub id: String,
    pub name: String,
    pub space: Arc<Mutex<Space>>,
    pub pieces: PerPiece<PieceBuilder>,
    pub colors: PerColor<()>,
}
impl PuzzleBuilder {
    /// Returns a builder for a puzzle that starts with an empty Euclidean space
    /// with no pieces.
    pub fn new_soup(name: String, id: String, ndim: u8) -> Self {
        PuzzleBuilder {
            name,
            id,
            space: Arc::new(Mutex::new(Space::new(ndim))),
            pieces: PerPiece::new(),
            colors: PerColor::new(),
        }
    }
    /// Returns a builder for a puzzle that starts with a single solid piece
    /// occupying all of Euclidean space.
    pub fn new_solid(name: String, id: String, ndim: u8) -> (Self, Piece) {
        let mut this = PuzzleBuilder::new_soup(name, id, ndim);
        this.pieces
            .push(PieceBuilder {
                shape: this.space.lock().whole_space(),
                stickers: vec![],
                is_active: true,
            })
            .unwrap();
        (this, Piece(0))
    }

    /// Cut each piece by a cut, throwing away the portions that are outside the
    /// cut. Every piece in the old set becomes inactive, and each piece in the
    /// new set inherits its active status from the corresponding piece in the
    /// old set.
    pub fn carve(&mut self, pieces: &PieceSet, cut_manifold: ManifoldRef) -> Result<PieceSet> {
        let mut space = self.space.lock();
        let mut cutter = space.carve(cut_manifold);
        let sticker_color = self.colors.push(())?;

        let old_pieces = pieces;
        let mut new_pieces = PieceSet::new();
        for piece in old_pieces.iter() {
            let old_piece = &mut self.pieces[piece];

            // Cut and deactivate piece.
            let result = cut_piece(old_piece, &mut cutter, Some(sticker_color))?;

            // Add new piece.
            if let Some(inside_piece) = result.inside {
                new_pieces.insert(self.pieces.push(inside_piece)?);
            }
        }

        Ok(new_pieces)
    }

    pub fn slice(&mut self, pieces: &PieceSet, cut_manifold: ManifoldRef) -> Result<PieceSet> {
        let mut space = self.space.lock();
        let mut cutter = space.slice(cut_manifold);

        let old_pieces = pieces;
        let mut new_pieces = PieceSet::new();
        for piece in old_pieces.iter() {
            let old_piece = &mut self.pieces[piece];

            // Cut and deactivate piece.
            let result = cut_piece(old_piece, &mut cutter, None)?;

            // Add new pieces.
            if let Some(inside_piece) = result.inside {
                new_pieces.insert(self.pieces.push(inside_piece)?);
            }
            if let Some(outside_piece) = result.outside {
                new_pieces.insert(self.pieces.push(outside_piece)?);
            }
        }

        Ok(new_pieces)
    }

    /// Performs the final steps of building a puzzle, generating the mesh and
    /// assigning IDs to pieces etc.
    pub fn build(self) -> Result<Arc<Puzzle>> {
        let space = self.space.lock();

        let twists = Arc::new(crate::PuzzleTwists {
            name: "Unknown".to_string(),
        });
        let mut mesh = MeshBuilder::new(space.ndim());

        for (_id, ()) in self.colors {
            mesh.add_color()
        }

        // As we construct the mesh, we'll renumber all the pieces and stickers
        // to exclude inactive ones.
        let mut pieces = PerPiece::new();
        let mut stickers = PerSticker::new();

        let mut simplexifier = Simplexifier::new(&space);
        for (_old_piece_id, mut piece) in self.pieces {
            if !piece.is_active {
                continue;
            }

            let piece_id = pieces.push(PieceInfo {
                stickers: smallvec![],
                piece_type: PieceType(0), // TODO
            })?;

            let piece_centroid_point = simplexifier.shape_centroid_point(piece.shape.id)?;
            let mut piece_mesh = mesh.add_piece(piece_centroid_point)?;
            piece.stickers.sort_unstable_by_key(|s| s.color);
            for sticker in piece.stickers {
                let sticker_id = stickers.push(StickerInfo {
                    piece: piece_id,
                    color: sticker.color,
                })?;
                pieces[piece_id].stickers.push(sticker_id);

                let manifold = space.manifold_of(sticker.shape).id;
                let sticker_centroid = simplexifier.shape_centroid(sticker.shape.id)?;
                let mut sticker_mesh =
                    piece_mesh.add_sticker(manifold, sticker.color, sticker_centroid)?;

                let mut queue = vec![sticker.shape];
                let mut seen = HashSet::new();

                while let Some(subshape_of_sticker) = queue.pop() {
                    match space[space[subshape_of_sticker.id].manifold].ndim {
                        0..=1 => continue, // should be unreachable
                        3.. => {
                            // TODO: handle non-flat shapes
                            for b in space.boundary_of(subshape_of_sticker) {
                                if seen.insert(b.id) {
                                    queue.push(b);
                                }
                            }
                        }
                        2 => {
                            let manifold = space.manifold_of(subshape_of_sticker);
                            let blade = space.blade_of(manifold);
                            let mut polygon_mesh =
                                sticker_mesh.add_polygon(&blade, sticker.color)?;
                            let tris = simplexifier.face_polygons(subshape_of_sticker)?;

                            let mut vertex_id_map: HashMap<VertexId, u32> = HashMap::new();
                            for old_id in tris.iter().flat_map(|&tri| tri) {
                                let vertex_pos = &simplexifier[old_id];
                                let sticker_shrink_vector = vector![]; // TODO
                                let new_id =
                                    polygon_mesh.add_vertex(vertex_pos, sticker_shrink_vector)?;
                                vertex_id_map.insert(old_id, new_id);
                            }

                            for tri in tris {
                                polygon_mesh.add_tri(tri.map(|i| vertex_id_map[&i]));
                            }
                        }
                    }
                }
            }
        }

        Ok(Arc::new_cyclic(|this| Puzzle {
            this: Weak::clone(this),
            name: self.name,
            id: self.id,

            mesh: mesh.finish(),

            pieces,
            stickers,
            piece_types: [PieceTypeInfo {
                name: "Piece".to_string(), // TODO piece types
            }]
            .into_iter()
            .collect(),

            scramble_moves_count: 500, // TODO

            notation: NotationScheme {},

            new: Box::new(PuzzleState::new),
        }))
    }
}

#[derive(Debug, Clone)]
pub struct PieceBuilder {
    pub shape: ShapeRef,
    pub stickers: Vec<StickerBuilder>,
    /// Whether the piece should be part of the final puzzle.
    pub is_active: bool,
}

#[derive(Debug, Clone)]
pub struct StickerBuilder {
    pub shape: ShapeRef,
    pub color: Color,
}

#[derive(Debug, Clone)]
struct PieceCutResult {
    inside: Option<PieceBuilder>,
    outside: Option<PieceBuilder>,
}

#[derive(Debug, Clone)]
struct StickerSetCutResult {
    inside: Vec<StickerBuilder>,
    outside: Vec<StickerBuilder>,
}

fn cut_piece(
    piece: &mut PieceBuilder,
    cutter: &mut CutInProgress,
    new_color: Option<Color>,
) -> Result<PieceCutResult> {
    let shape_cut_result = cutter.cut(piece.shape).context("cutting piece")?;

    // Cut existing stickers.
    let mut stickers_cut_result = cut_sticker_set(&piece.stickers, cutter)?;

    // Add new sticker.
    if let Some(new_color) = new_color {
        if let Some(new_sticker_shape) = shape_cut_result.flush_facet {
            stickers_cut_result.inside.push(StickerBuilder {
                shape: new_sticker_shape,
                color: new_color,
            });
            stickers_cut_result.outside.push(StickerBuilder {
                shape: -new_sticker_shape,
                color: new_color,
            });
        }
    }

    // Construct pieces.
    let inside = match shape_cut_result.inside {
        Some(inside_shape) => Some(PieceBuilder {
            shape: inside_shape,
            stickers: stickers_cut_result.inside,
            is_active: piece.is_active,
        }),
        None => None,
    };
    let outside = match shape_cut_result.outside {
        Some(outside_shape) => Some(PieceBuilder {
            shape: outside_shape,
            stickers: stickers_cut_result.outside,
            is_active: piece.is_active,
        }),
        None => None,
    };

    // Set old piece to inactive.
    piece.is_active = false;

    Ok(PieceCutResult { inside, outside })
}

fn cut_sticker_set(
    sticker_set: &[StickerBuilder],
    cutter: &mut CutInProgress,
) -> Result<StickerSetCutResult> {
    let mut inside = vec![];
    let mut outside = vec![];
    for old_sticker in sticker_set {
        let result = cutter.cut(old_sticker.shape).context("cutting sticker")?;
        if let Some(inside_shape) = result.inside {
            inside.push(StickerBuilder {
                shape: inside_shape,
                color: old_sticker.color,
            });
        }
        if let Some(outside_shape) = result.outside {
            outside.push(StickerBuilder {
                shape: outside_shape,
                color: old_sticker.color,
            });
        }
    }
    Ok(StickerSetCutResult { inside, outside })
}

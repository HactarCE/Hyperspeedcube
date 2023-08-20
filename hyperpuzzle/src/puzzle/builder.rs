use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Weak};

use anyhow::Result;
use hypermath::prelude::*;
use hypershape::prelude::*;
use itertools::Itertools;
use parking_lot::Mutex;
use smallvec::smallvec;
use tinyset::Set64;

use super::simplices::{Simplexifier, VertexId};
use super::{
    Color, MeshBuilder, NotationScheme, PerPiece, PerSticker, Piece, PieceInfo, PieceType,
    PieceTypeInfo, Puzzle, PuzzleState, StickerInfo,
};

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct PieceSet(pub Set64<Piece>);

#[derive(Debug)]
pub struct PuzzleBuilder {
    pub name: String,
    pub space: Arc<Mutex<Space>>,
    pieces: PerPiece<PieceBuilder>,
}
impl PuzzleBuilder {
    /// Returns a builder for a puzzle that starts with an empty Euclidean space
    /// with no pieces.
    pub fn new_soup(name: String, ndim: u8) -> Self {
        PuzzleBuilder {
            name,
            space: Arc::new(Mutex::new(Space::new(ndim))),
            pieces: PerPiece::new(),
        }
    }
    /// Returns a builder for a puzzle that starts with a single solid piece
    /// occupying all of Euclidean space.
    pub fn new_solid(name: String, ndim: u8) -> (Self, Piece) {
        let mut this = PuzzleBuilder::new_soup(name, ndim);
        this.pieces
            .push(PieceBuilder {
                shape: this.space.lock().whole_space(),
                stickers: vec![],
                is_active: true,
            })
            .unwrap();
        (this, Piece(0))
    }

    pub(crate) fn take(&mut self) -> Self {
        let default = Self::new_soup(self.name.clone(), self.space.lock().ndim());
        std::mem::replace(self, default)
    }

    pub fn build(self) -> Result<Arc<Puzzle>> {
        let space = self.space.lock();

        let shape = Arc::new(crate::PuzzleShape {
            name: "Unknown".to_string(),
        });
        let twists = Arc::new(crate::PuzzleTwists {
            name: "Unknown".to_string(),
        });
        let mut mesh = MeshBuilder::new(space.ndim());

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
            pieces[piece_id].stickers = piece
                .stickers
                .iter()
                .map(|sticker| {
                    stickers.push(StickerInfo {
                        piece: piece_id,
                        color: sticker.color,
                    })
                })
                .try_collect()?;

            let piece_centroid_point = simplexifier.shape_centroid_point(piece.shape.id)?;
            let mut piece_mesh = mesh.add_piece(piece_centroid_point)?;
            piece.stickers.sort_unstable_by_key(|s| s.color);
            for sticker in piece.stickers {
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
                            let mut polygon_mesh = sticker_mesh.add_polygon(&blade)?;
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
            name: self.name.clone(),
            shape,
            twists,

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

#[derive(Debug)]
struct PieceBuilder {
    shape: ShapeRef,
    stickers: Vec<StickerBuilder>,
    /// Whether the piece should be part of the final puzzle.
    is_active: bool,
}

#[derive(Debug)]
struct StickerBuilder {
    shape: ShapeRef,
    color: Color,
}

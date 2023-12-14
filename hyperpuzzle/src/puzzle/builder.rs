use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Weak};

use eyre::{Result, WrapErr};
use hypermath::prelude::*;
use hypershape::prelude::*;
use itertools::Itertools;
use parking_lot::Mutex;
use smallvec::smallvec;

use super::simplices::{Simplexifier, VertexId};
use super::{
    Color, MeshBuilder, MeshStickerBuilder, Notation, PerColor, PerPiece, PerSticker, Piece,
    PieceInfo, PieceSet, PieceType, PieceTypeInfo, Puzzle, PuzzleState, StickerInfo,
};

/// Puzzle being constructed.
#[derive(Debug)]
pub struct PuzzleBuilder {
    /// Puzzle ID.
    pub id: String,
    /// Name of the puzzle.
    pub name: String,

    /// Space where the puzzle exists.
    pub space: Arc<Mutex<Space>>,
    /// Puzzle pieces.
    pub pieces: PerPiece<PieceBuilder>,
    /// Sticker colors.
    pub colors: PerColor<ColorBuilder>,
}
impl PuzzleBuilder {
    /// Returns a builder for a puzzle that starts with an empty Euclidean space
    /// with no pieces.
    pub fn new_soup(name: String, id: String, ndim: u8) -> Result<Self> {
        Ok(PuzzleBuilder {
            name,
            id,
            space: Arc::new(Mutex::new(Space::new(ndim)?)),
            pieces: PerPiece::new(),
            colors: PerColor::new(),
        })
    }
    /// Returns a builder for a puzzle that starts with a single solid piece
    /// occupying all of Euclidean space.
    pub fn new_solid(name: String, id: String, ndim: u8) -> Result<(Self, Piece)> {
        let mut this = PuzzleBuilder::new_soup(name, id, ndim)?;
        this.pieces.push(PieceBuilder {
            shape: this.space.lock().whole_space(),
            is_active: true,
        })?;
        Ok((this, Piece(0)))
    }

    /// Cuts each piece by a cut, throwing away the portions that are outside
    /// the cut. Every piece in the old set becomes inactive, and each piece in
    /// the new set inherits its active status from the corresponding piece in
    /// the old set.
    pub fn carve(&mut self, pieces: &PieceSet, cut_manifold: ManifoldRef) -> Result<PieceSet> {
        let mut cut = AtomicCut::carve(cut_manifold);
        self.cut_and_deactivate_pieces(&mut cut, pieces)
    }

    /// Cuts each piece by a cut, keeping all results. Every piece in the old
    /// set becomes inactive, and each piece in the new set inherits its active
    /// status from the corresponding piece in the old set.
    pub fn slice(&mut self, pieces: &PieceSet, cut_manifold: ManifoldRef) -> Result<PieceSet> {
        let mut cut = AtomicCut::slice(cut_manifold);
        self.cut_and_deactivate_pieces(&mut cut, pieces)
    }

    /// Returns the set of active pieces.
    pub fn active_pieces(&self) -> PieceSet {
        self.pieces
            .iter()
            .filter(|(_id, piece)| piece.is_active)
            .map(|(id, _piece)| id)
            .collect()
    }

    fn cut_and_deactivate_pieces(
        &mut self,
        cut: &mut AtomicCut,
        pieces: &PieceSet,
    ) -> Result<PieceSet> {
        let mut space = self.space.lock();

        let old_pieces = pieces;
        let mut new_pieces = PieceSet::new();
        for piece in old_pieces.iter() {
            let old_piece = &mut self.pieces[piece];

            // Cut and deactivate piece.
            for new_piece in cut_and_deactivate_piece(&mut space, old_piece, cut)? {
                let new_piece_id = self.pieces.push(new_piece)?;
                new_pieces.insert(new_piece_id);
            }
        }

        Ok(new_pieces)
    }

    /// Performs the final steps of building a puzzle, generating the mesh and
    /// assigning IDs to pieces etc.
    pub fn build(self) -> Result<Arc<Puzzle>> {
        let space = self.space.lock();

        let mut mesh = MeshBuilder::new(space.ndim());

        for _ in &self.colors {
            mesh.add_color();
        }
        let stickered_manifolds: HashMap<ManifoldRef, Color> = self
            .colors
            .into_iter()
            .map(|(color_id, color_builder)| (color_builder.manifold, color_id))
            .collect();

        // As we construct the mesh, we'll renumber all the pieces and stickers
        // to exclude inactive ones.
        let mut pieces = PerPiece::new();
        let mut stickers = PerSticker::new();

        let mut simplexifier = Simplexifier::new(&space);
        for (_old_piece_id, piece) in self.pieces {
            if !piece.is_active {
                continue;
            }

            // Add piece to mesh.
            let piece_id = pieces.push(PieceInfo {
                stickers: smallvec![],
                piece_type: PieceType(0), // TODO
            })?;
            let piece_centroid_point = simplexifier.shape_centroid_point(piece.shape.id)?;
            let mut piece_mesh = mesh.add_piece(piece_centroid_point)?;

            // Add stickers to mesh.
            let piece_stickers = space
                .boundary_of(piece.shape)
                .map(|sticker_shape| {
                    let facet_manifold = space.manifold_of(sticker_shape);
                    let color = *stickered_manifolds
                        .get(&facet_manifold)
                        .unwrap_or(&Color::INTERNAL);
                    (color, sticker_shape)
                })
                .sorted();
            for (color, sticker_shape) in piece_stickers {
                let sticker_id = stickers.push(StickerInfo {
                    piece: piece_id,
                    color,
                })?;
                pieces[piece_id].stickers.push(sticker_id);

                let manifold = space.manifold_of(sticker_shape).id;
                let sticker_centroid = simplexifier.shape_centroid(sticker_shape.id)?;
                let mut sticker_mesh = piece_mesh.add_sticker(manifold, color, sticker_centroid)?;

                build_sticker_mesh(&space, &mut simplexifier, &mut sticker_mesh, sticker_shape)?;
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

            notation: Notation {},

            new: Box::new(PuzzleState::new),
        }))
    }
}

/// Piece of a puzzle during puzzle construction.
#[derive(Debug, Clone)]
pub struct PieceBuilder {
    /// Shape of the piece.
    pub shape: AtomicPolytopeRef,
    /// Whether the piece should be part of the final puzzle.
    pub is_active: bool,
}

/// Sticker color during puzzle construction.
#[derive(Debug, Clone)]
pub struct ColorBuilder {
    /// Manifold of the color; stickers flush with this manifold will be
    /// assigned this color.
    pub manifold: ManifoldRef,
}

fn cut_and_deactivate_piece(
    space: &mut Space,
    piece: &mut PieceBuilder,
    cut: &mut AtomicCut,
) -> Result<Vec<PieceBuilder>> {
    // Deactivate old piece.
    let is_active = std::mem::replace(&mut piece.is_active, false);

    Ok(space
        .cut_atomic_polytope_set([piece.shape].into_iter().collect(), cut)
        .context("error cutting piece")?
        .into_iter()
        .map(|shape| PieceBuilder { shape, is_active })
        .collect())
}

fn build_sticker_mesh(
    space: &Space,
    simplexifier: &mut Simplexifier<'_>,
    sticker_mesh: &mut MeshStickerBuilder<'_, '_>,
    sticker_shape: AtomicPolytopeRef,
) -> Result<()> {
    let mut queue = vec![sticker_shape];
    let mut seen = HashSet::new();

    while let Some(subshape_of_sticker) = queue.pop() {
        match space.ndim_of(subshape_of_sticker) {
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
                    let new_id = polygon_mesh.add_vertex(vertex_pos, sticker_shrink_vector)?;
                    vertex_id_map.insert(old_id, new_id);
                }

                for tri in tris {
                    polygon_mesh.add_tri(tri.map(|i| vertex_id_map[&i]));
                }
            }
        }
    }

    Ok(())
}

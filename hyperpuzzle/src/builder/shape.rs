use std::sync::{Arc, Weak};

use eyre::{Context, Result};
use hypershape::prelude::*;
use itertools::Itertools;
use parking_lot::Mutex;

use super::{ColorSystemBuilder, PieceBuilder};
use crate::puzzle::*;

/// Soup of shapes being constructed.
#[derive(Debug)]
pub struct ShapeBuilder {
    /// Reference-counted pointer to this struct.
    pub this: Weak<Mutex<Self>>,

    /// Shape ID.
    pub id: Option<String>,

    /// Space where the puzzle exists.
    pub space: Arc<Mutex<Space>>,

    /// Symmetry group of the whole shape.
    pub symmetry: Option<SchlafliSymbol>,

    /// Puzzle pieces.
    pub pieces: PerPiece<PieceBuilder>,
    /// Pieces that are not defunct (removed or cut) and so should be included
    /// in the final puzzle.
    pub active_pieces: PieceSet,

    /// Facet colors.
    pub colors: ColorSystemBuilder,
}
impl ShapeBuilder {
    /// Constructs a shape builder that starts with an empty Euclidean space.
    pub fn new_empty(id: Option<String>, space: Arc<Mutex<Space>>) -> Result<Arc<Mutex<Self>>> {
        Ok(Arc::new_cyclic(|this| {
            Mutex::new(Self {
                this: this.clone(),

                id,

                space,

                symmetry: None,

                pieces: PerPiece::new(),
                active_pieces: PieceSet::new(),

                colors: ColorSystemBuilder::new(),
            })
        }))
    }

    /// Constructs a shape builder that starts with a single solid piece
    /// occupying all of Euclidean space.
    pub fn new_full(id: Option<String>, space: Arc<Mutex<Space>>) -> Result<Arc<Mutex<Self>>> {
        let this = Self::new_empty(id, Arc::clone(&space))?;
        let mut this_guard = this.lock();
        let root_piece_builder = PieceBuilder::new(space.lock().whole_space());
        let root_piece = this_guard.pieces.push(root_piece_builder)?;
        this_guard.active_pieces.insert(root_piece);
        drop(this_guard);
        Ok(this)
    }

    /// Returns an `Arc` reference to the shape builder.
    pub fn arc(&self) -> Arc<Mutex<Self>> {
        self.this
            .upgrade()
            .expect("`ShapeBuilder` removed from `Arc`")
    }

    /// Returns the number of dimensions of the underlying space.
    pub fn ndim(&self) -> u8 {
        self.space.lock().ndim()
    }

    /// Returns a deep copy of the shape. This is a relatively expensive
    /// operation.
    pub fn clone(&self, space: &Arc<Mutex<Space>>) -> Result<Arc<Mutex<Self>>> {
        let old_space = self.space.lock();
        let mut new_space = space.lock();
        let mut map = SpaceMap::new(&old_space, &mut new_space)?;

        let pieces: PerPiece<PieceBuilder> = self
            .active_pieces
            .iter()
            .map(|piece| PieceBuilder {
                polytope: map.map(self.pieces[piece].polytope),
                cut_result: PieceSet::new(),
            })
            .collect();
        let active_pieces = pieces.iter_keys().collect();

        Ok(Arc::new_cyclic(|this| {
            Mutex::new(Self {
                this: this.clone(),

                id: self.id.clone(),

                space: Arc::clone(space),

                symmetry: self.symmetry.clone(),
                pieces,
                active_pieces,
                colors: self.colors.clone(&mut map),
            })
        }))
    }

    /// Cuts each piece by a cut, throwing away the portions that are outside
    /// the cut. Each piece in the old set becomes defunct, and each piece in
    /// the new set inherits its active status from the corresponding piece in
    /// the old set.
    ///
    /// If `pieces` is `None`, then it is assumed to be all active pieces.
    pub fn carve(&mut self, pieces: Option<&PieceSet>, cut_manifold: ManifoldRef) -> Result<()> {
        let mut cut = AtomicCut::carve(cut_manifold);
        self.cut_and_deactivate_pieces(&mut cut, pieces)
    }
    /// Cuts each piece by a cut, keeping all results. Each piece in the old set
    /// becomes defunct, and each piece in the new set inherits its active
    /// status from the corresponding piece in the old set.
    ///
    /// If `pieces` is `None`, then it is assumed to be all active pieces.
    pub fn slice(&mut self, pieces: Option<&PieceSet>, cut_manifold: ManifoldRef) -> Result<()> {
        let mut cut = AtomicCut::slice(cut_manifold);
        self.cut_and_deactivate_pieces(&mut cut, pieces)
    }
    fn cut_and_deactivate_pieces(
        &mut self,
        cut: &mut AtomicCut,
        pieces: Option<&PieceSet>,
    ) -> Result<()> {
        let pieces = match pieces {
            Some(piece_set) => self.update_piece_set(piece_set),
            None => self.active_pieces.clone(),
        };

        let mut space = self.space.lock();

        for old_piece in pieces.iter() {
            // Cut the old piece and add the new pieces as active.
            let new_piece_polytopes = space
                .cut_atomic_polytope_set(
                    [self.pieces[old_piece].polytope].into_iter().collect(),
                    cut,
                )
                .context("error cutting piece")?;
            let new_pieces: PieceSet = new_piece_polytopes
                .into_iter()
                .map(|new_piece_polytope| {
                    let new_piece = self.pieces.push(PieceBuilder::new(new_piece_polytope))?;
                    self.active_pieces.insert(new_piece);
                    eyre::Ok(new_piece)
                })
                .try_collect()?;
            self.active_pieces.extend(new_pieces.iter());

            // The old piece is defunct, so deactivate it and record its cut
            // result.
            self.active_pieces.remove(&old_piece);
            self.pieces[old_piece].cut_result = new_pieces;
        }

        Ok(())
    }

    /// Updates a piece set, replacing defunct pieces with their cut results.
    /// Call this before doing anything with a piece set to prevent operating on
    /// defunct pieces.
    pub fn update_piece_set(&self, piece_set: &PieceSet) -> PieceSet {
        let mut queue = piece_set.iter().collect_vec();
        let mut output = PieceSet::new();
        while let Some(old_piece) = queue.pop() {
            if self.active_pieces.contains(&old_piece) {
                output.insert(old_piece);
            } else {
                queue.extend(self.pieces[old_piece].cut_result.iter());
            }
        }
        output
    }
}

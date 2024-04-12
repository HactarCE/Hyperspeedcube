use std::sync::Arc;

use hypermath::prelude::*;
use hypershape::prelude::*;

use crate::{Axis, LayerMask, PerPiece, Piece, Puzzle};

/// Instance of a puzzle with a particular state.
#[derive(Debug, Clone)]
pub struct PuzzleState {
    /// Immutable puzzle type info.
    puzzle_type: Arc<Puzzle>,
    /// Position and rotation of each piece.
    piece_transforms: PerPiece<Isometry>,
}
impl PuzzleState {
    /// Constructs a new instance of a puzzle.
    pub fn new(puzzle_type: Arc<Puzzle>) -> Self {
        let piece_transforms = puzzle_type.pieces.map_ref(|_, _| Isometry::ident());
        PuzzleState {
            puzzle_type,
            piece_transforms,
        }
    }
    /// Returns the puzzle type
    pub fn ty(&self) -> &Puzzle {
        &self.puzzle_type
    }
    /// Returns the position and rotation of each piece.
    pub fn piece_transforms(&self) -> &PerPiece<Isometry> {
        &self.piece_transforms
    }

    pub fn pieces_in_grip(&self, axis: Axis, layer: LayerMask) -> Vec<Piece> {
        todo!("pieces_in_grip")
    }
}

use std::sync::Arc;

use hypermath::prelude::*;

use crate::{PerPiece, Puzzle};

pub struct PuzzleState {
    puzzle_type: Arc<Puzzle>,
    piece_rotations: PerPiece<Isometry>,
}
impl PuzzleState {
    pub fn new(puzzle_type: Arc<Puzzle>) -> Self {
        let piece_rotations = puzzle_type.pieces.map_ref(|_, _| Isometry::ident());
        PuzzleState {
            puzzle_type,
            piece_rotations,
        }
    }
}

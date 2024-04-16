use std::sync::Arc;

use hypermath::Isometry;
use hyperpuzzle::{LayerMask, PerPiece, Puzzle, PuzzleState, Twist};

#[derive(Debug, Clone)]
pub struct PuzzleController {
    puzzle_state: PuzzleState,
}
impl PuzzleController {
    pub fn new(puzzle: &Arc<Puzzle>) -> Self {
        Self {
            puzzle_state: PuzzleState::new(Arc::clone(puzzle)),
        }
    }

    pub fn puzzle_type(&self) -> &Arc<Puzzle> {
        self.puzzle_state.ty()
    }

    pub fn peice_transforms(&self) -> PerPiece<Isometry> {
        self.puzzle_state.piece_transforms()
    }

    pub fn do_twist(&mut self, twist: Twist) {
        self.puzzle_state.do_twist(twist, LayerMask(1));
    }
}

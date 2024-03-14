use std::sync::Arc;

use hypermath::Isometry;
use hyperpuzzle::Puzzle;

#[derive(Debug, Clone)]
pub struct PuzzleController {
    pub puzzle: Arc<Puzzle>,
}
impl PuzzleController {
    pub fn new(puzzle: &Arc<Puzzle>) -> Self {
        Self {
            puzzle: Arc::clone(puzzle),
        }
    }
}

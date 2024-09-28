use std::collections::HashMap;
use std::sync::Arc;

use crate::lua::{PuzzleGenerator, PuzzleParams};
use crate::Puzzle;

/// Puzzle defined in a Lua file that is cached when constructed.
#[derive(Debug)]
pub(crate) struct LazyPuzzle {
    /// Parameters to construct the puzzle.
    pub params: Arc<PuzzleParams>,
    /// Cached constructed puzzle.
    pub constructed: Option<Arc<Puzzle>>,
}
impl LazyPuzzle {
    /// Returns a new lazy puzzle that has not yet been constructed.
    pub fn new(params: PuzzleParams) -> Self {
        Self {
            params: Arc::new(params),
            constructed: None,
        }
    }
}

/// Puzzle generator defined in a Lua file whose puzzles are cached whenever
/// they are constructed.
#[derive(Debug)]
pub(crate) struct LazyPuzzleGenerator {
    /// Generator to construct a puzzle.
    pub generator: Arc<PuzzleGenerator>,
    /// Cached constructed puzzles.
    pub constructed: HashMap<String, Arc<Puzzle>>,
}
impl LazyPuzzleGenerator {
    /// Returns a new lazy puzzle generator that has not yet been constructed.
    pub fn new(generator: PuzzleGenerator) -> Self {
        Self {
            generator: Arc::new(generator),
            constructed: HashMap::new(),
        }
    }
}

use std::sync::Arc;

use crate::lua::PuzzleParams;
use crate::Puzzle;

/// Puzzle defined in a Lua file that is cached when constructed.
#[derive(Debug)]
pub(crate) struct LazyPuzzle {
    /// Parameters to construct the object.
    pub params: Arc<PuzzleParams>,
    /// Cached constructed object.
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

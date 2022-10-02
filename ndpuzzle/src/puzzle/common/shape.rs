use super::*;

/// Puzzle shape metadata.
#[derive(Debug)]
pub struct PuzzleShape {
    /// Shape name.
    pub name: String,
    /// Number of dimensions.
    pub ndim: u8,
    /// Puzzles faces.
    // TODO: rename to `facets` and `FacetInfo`
    pub faces: Vec<FaceInfo>,
}
impl_puzzle_info_trait!(for PuzzleShape { fn info(Face) -> &FaceInfo { .faces } });

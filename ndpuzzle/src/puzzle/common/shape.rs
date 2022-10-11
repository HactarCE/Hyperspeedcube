use super::*;

/// Puzzle shape metadata.
#[derive(Debug)]
pub struct PuzzleShape {
    /// Shape name.
    pub name: Option<String>,
    /// Number of dimensions.
    pub ndim: u8,
    /// Puzzles facets.
    pub facets: Vec<FacetInfo>,
    /// Distance from origin to outermost point.
    pub radius: f32,
}
impl_puzzle_info_trait!(for PuzzleShape { fn info(Facet) -> &FacetInfo { .facets } });

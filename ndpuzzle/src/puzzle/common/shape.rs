use ahash::AHashMap;

use super::*;

/// Puzzle shape metadata.
#[derive(Debug)]
pub struct PuzzleShape {
    /// Shape name.
    pub name: Option<String>,
    /// Number of dimensions.
    pub ndim: u8,
    /// Facets.
    pub facets: Vec<FacetInfo>,
    /// Canonical ordering of facets.
    pub facet_order: Vec<Facet>,
    /// Distance from origin to outermost point.
    pub radius: Float,

    /// Facets listed by name.
    pub facets_by_name: AHashMap<String, Facet>,
}
impl_puzzle_info_trait!(for PuzzleShape { fn info(Facet) -> &FacetInfo { .facets } });

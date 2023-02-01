use super::Schlafli;
use crate::math::Mobius;
use cgmath::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub struct Tiling {
    config: TilingConfig,
    tiles: Vec<Tile>,
}
impl Tiling {
    pub fn generate(config: TilingConfig) -> Self {
        todo!()
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub enum PolytopeProjection {
    #[default]
    FaceCentered,
    VertexCentered,
    EdgeCentered,
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Tile {}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct TilingConfig {
    schlafli: Schlafli,

    /// Mobius transformation to apply when creating the tiling.
    m: Mobius,

    /// Maximum number of tiles.
    max_tiles: usize,

    /// Shrinkage to apply to the drawn portion of a tile.
    ///
    /// Default is 1.0 (no shrinkage).
    shrink: f64,
}
impl TilingConfig {
    pub fn new(p: u8, q: u8, max_tiles: usize) -> Self {
        Self {
            schlafli: Schlafli::new(p, q),

            m: Mobius::one(),

            max_tiles,

            shrink: 1.0,
        }
    }
    pub fn spherical(p: u8, q: u8) -> Option<Self> {
        let max_tiles = platonic_solid_faces(p, q)?;
        Some(Self::new(p, q, max_tiles))
    }
}

pub fn platonic_solid_faces(p: u8, q: u8) -> Option<usize> {
    match (p, q) {
        (3, 3) => Some(4),  // tetrahedron
        (4, 3) => Some(6),  // cube
        (5, 3) => Some(12), // dodecahedron
        (3, 4) => Some(8),  // octahedron
        (3, 5) => Some(20), // icosahedron
        _ => None,
    }
}

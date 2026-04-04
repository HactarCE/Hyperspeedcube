//! Multidimensional shape slicing and other geometric algorithms.

pub mod flat;

pub use flat::*;

/// Structs, traits, and constants.
pub mod prelude {
    pub use crate::flat::*;
}

/// Radius of the promordial cube, which determines the maximum extent of all
/// vertices along any axis.
///
/// This must be large enough that it contains all geomoetry, but should be
/// relatively small to improve precision.
pub const PRIMORDIAL_CUBE_RADIUS: hypermath::Float = 64.0; // big power of 2 feels good

//! Multidimensional shape slicing and other geometric algorithms.

pub mod flat;

pub use flat::*;

/// Structs, traits, and constants.
pub mod prelude {
    pub use crate::flat::*;
}

/// Radius of the promordial cube, which determines the maximum extent of all
/// vertices along any axis.
pub const PRIMORDIAL_CUBE_RADIUS: hypermath::Float = 1_048_576.0; // big power of 2 feels good

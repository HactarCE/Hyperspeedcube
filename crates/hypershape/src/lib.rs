//! Multidimensional shape slicing and other geometric algorithms.

pub mod conformal;
pub mod flat;
pub mod group;
mod slabmap;
mod util;

pub use flat::*;
pub use group::*;
use slabmap::SlabMap;

/// Structs, traits, and constants.
pub mod prelude {
    pub use crate::flat::*;
    pub use crate::group::*;
}

#[cfg(test)]
mod tests;

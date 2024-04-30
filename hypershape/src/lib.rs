//! Multidimensional shape slicing and other geometric algorithms.

pub mod conformal;
pub mod flat;
pub mod group;
mod simplicial;
mod slabmap;
mod util;

pub use flat::*;
pub use group::*;
pub use simplicial::*;
use slabmap::SlabMap;

/// Structs, traits, and constants.
pub mod prelude {
    pub use crate::flat::*;
    pub use crate::group::*;
    pub use crate::simplicial::*;
}

#[cfg(test)]
mod tests;

//! Multidimensional shape slicing and other geometric algorithms.

pub mod flat;

pub use flat::*;

/// Structs, traits, and constants.
pub mod prelude {
    pub use crate::flat::*;
}

/// Default radius for the promordial cube, which determines the maximum extent
/// of all vertices along any axis.
///
/// This must be large enough that it contains all geomoetry, but should be
/// relatively small to improve precision.
pub const DEFAULT_PRIMORDIAL_CUBE_RADIUS: hypermath::Float = 64.0; // big power of 2 feels good

/// Recommended radius for the primordial cube, given the maximum facet pole
/// distance from the origin.
///
/// This can only be an estimate because the primordial cube radius should
/// really be based on the maximum _vertex_ distance from the origin. But this
/// is guaranteed to work for regular simplices, which are generally the
/// pointiest shape we care about.
pub fn recommended_primordial_cube_radius(
    ndim: u8,
    max_distance: hypermath::Float,
) -> hypermath::Float {
    (1 << ((max_distance * ndim as hypermath::Float).log2().floor() as i32 + 2)) as _
}

/// Default limit for the number of polytopes that can result from unfolding one
/// polytope.
pub const DEFAULT_UNFOLD_LIMIT: usize = 100_000;

//! Multidimensional vector, matrix, and conformal geometric algebra primitives.

pub use {approx, num_traits as num, smallvec};

/// Floating-point type used for geometry (either `f32` or `f64`).
pub type Float = f64;

/// Small floating-point value used for comparisons and tiny offsets.
pub const EPSILON: Float = 0.000001;

/// Names for axes up to 10 dimensions.
pub const AXIS_NAMES: &str = "XYZWVUTSRQ";

/// Maximum number of dimensions, which defaults to 10.
pub const MAX_NDIM: u8 = 10;

/// Asserts that both arguments are approximately equal.
#[macro_export]
macro_rules! assert_approx_eq {
    ($a:expr, $b:expr $(,)?) => {
        $crate::approx::assert_abs_diff_eq!($a, $b, epsilon = $crate::EPSILON)
    };
}

#[macro_use]
mod impl_macros;
#[macro_use]
mod vector;
#[macro_use]
pub mod collections;

pub mod approx_cmp;
pub mod centroid;
pub mod cga;
pub mod hyperplane;
pub mod matrix;
pub mod permutations;
pub mod pga;
pub mod sign;
pub mod util;
pub mod which_side;

pub use sign::Sign;
pub use which_side::PointWhichSide;

/// Structs, traits, and constants (excluding [`crate::collections`]).
pub mod prelude {
    pub use crate::approx_cmp::*;
    pub use crate::centroid::Centroid;
    pub use crate::collections::{ApproxHashMap, ApproxHashMapKey, IndexOutOfRange, IndexOverflow};
    pub use crate::hyperplane::*;
    pub use crate::matrix::*;
    pub use crate::permutations::{self, Parity};
    pub use crate::sign::Sign;
    pub use crate::traits::*;
    pub use crate::vector::*;
    pub use crate::which_side::*;
    pub use crate::{cga, pga, vector, Float, AXIS_NAMES, EPSILON, MAX_NDIM};
}
pub use prelude::*;

/// Traits only.
pub mod traits {
    pub use approx::AbsDiffEq;
    pub use tinyset::Fits64;

    pub use crate::cga::{AsMultivector, ToConformalPoint};
    pub use crate::collections::{ApproxHashMapKey, IndexNewtype, VecMap};
    pub use crate::pga::TransformByMotor;
    pub use crate::util::IterWithExactSizeExt;
    pub use crate::vector::VectorRef;
}

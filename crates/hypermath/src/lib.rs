//! Multidimensional vector, matrix, and conformal geometric algebra primitives.

pub use {approx_collections, num_traits as num, smallvec};

/// Floating-point type used for geometry (either `f32` or `f64`).
pub type Float = f64;

/// Small floating-point value used for comparisons and tiny offsets.
pub const EPSILON: Float = 0.000001;

/// Names for axes up to 7 dimensions.
pub const AXIS_NAMES: &str = "XYZWVUT";

/// Returns the axis number for a character.
pub fn axis_from_char(c: char) -> Option<u8> {
    AXIS_NAMES.find(c.to_ascii_uppercase()).map(|i| i as u8)
}

/// Maximum number of dimensions.
pub const MAX_NDIM: u8 = 7;

/// Asserts that both arguments are approximately equal.
#[macro_export]
macro_rules! assert_approx_eq {
    ($a:expr, $b:expr $(,)?) => {
        match (&$a, &$b) {
            (a, b) => {
                assert!(
                    $crate::APPROX.eq(a, b),
                    "assert_approx_eq!({}, {})

    left  = {:?}
    right = {:?}

",
                    stringify!(a),
                    stringify!(b),
                    a,
                    b,
                );
            }
        }
    };
}

macro_rules! debug_panic {
    ($($tok:tt)*) => {
        match cfg!(debug_assertions) {
            true => panic!($($tok)*),
            false => log::error!($($tok)*),
        }
    };
}

#[macro_use]
mod impl_macros;
#[macro_use]
mod vector;
#[macro_use]
mod point;
#[macro_use]
pub mod collections;

pub mod centroid;
pub mod hyperplane;
pub mod matrix;
pub mod permutations;
pub mod pga;
pub mod sign;
pub mod util;
pub mod which_side;

pub use sign::Sign;
pub use which_side::PointWhichSide;

/// Approximate comparison tuned for doing geometric puzzle computations.
pub const APPROX: Precision = Precision::DEFAULT;

/// Structs, traits, and constants (excluding [`crate::collections`]).
pub mod prelude {
    pub use approx_collections::{self, ApproxHashMap, FloatPool, Precision};

    pub use crate::centroid::Centroid;
    pub use crate::collections::{
        GenericVec, IndexOutOfRange, IndexOverflow, MotorNearestNeighborMap, VecMap,
    };
    pub use crate::hyperplane::*;
    pub use crate::matrix::*;
    pub use crate::permutations::{self, Parity};
    pub use crate::point::*;
    pub use crate::sign::Sign;
    pub use crate::traits::*;
    pub use crate::vector::*;
    pub use crate::which_side::*;
    pub use crate::{APPROX, AXIS_NAMES, EPSILON, Float, MAX_NDIM, pga, point, vector};
}
pub use prelude::*;

/// Traits only.
pub mod traits {
    pub use approx_collections::traits::*;
    pub use tinyset::Fits64;

    pub use crate::collections::IndexNewtype;
    pub use crate::pga::TransformByMotor;
    pub use crate::util::IterWithExactSizeExt;
    pub use crate::vector::VectorRef;
}

/// Returns `f` as an integer if it is approximately equal to one.
pub fn to_approx_integer(f: Float) -> Option<i64> {
    Some(f as i64).filter(|&i| APPROX.eq(f, i as Float))
}

/// Returns `f` as an unsigned integer if it is approximately equal to one.
pub fn to_approx_unsigned_integer(f: Float) -> Option<u64> {
    Some(f as u64).filter(|&i| APPROX.eq(f, i as Float))
}

use cgmath::{abs_diff_eq, prelude::*, AbsDiffEq, Matrix4};
use num_complex::Complex;

pub mod mobius;

pub use mobius::Mobius;

pub const EPSILON: f64 = 0.000001;

/// Orthonormalize using the Gram-Schmidt process.
///
/// https://en.wikipedia.org/wiki/Gram%E2%80%93Schmidt_process#Algorithm
#[must_use]
pub fn orthonormalize(mut u: Matrix4<f64>) -> Matrix4<f64> {
    u[0] = u[0].normalize();

    for i in 1..4 {
        for j in 0..(i - 1) {
            u[i] = u[i] - u[j] * u[j].dot(u[i]);
        }
        u[i] = u[i].normalize();
    }

    u
}

/// Assertion that delegates to [`approx_eq`], and panics with a helpful error
/// on failure.
macro_rules! debug_assert_approx_eq {
    ($a:expr, $b:expr) => {
        cgmath::assert_abs_diff_eq!(&$a, &$b, epsilon = crate::math::EPSILON)
    };
}

/// Returns whether two numbers are separated by less than `EPSILON`.
pub fn approx_eq<T: AbsDiffEq<Epsilon = f64>>(a: T, b: T) -> bool {
    abs_diff_eq!(&a, &b, epsilon = EPSILON)
}

/// Compares two numbers, but considers them equal if they are separated by less
/// than `EPSILON`.
pub fn approx_cmp<T: Copy + AbsDiffEq<Epsilon = f64> + PartialOrd>(
    a: T,
    b: T,
) -> std::cmp::Ordering {
    if approx_eq(a, b) {
        std::cmp::Ordering::Equal
    } else if a < b {
        std::cmp::Ordering::Less
    } else {
        std::cmp::Ordering::Greater
    }
}

/// Linearly interpolates (unclamped) between two points.
pub fn lerp<T: cgmath::EuclideanSpace<Scalar = f64>>(a: T, b: T, t: f64) -> T {
    a + (b - a) * t
}

/// Returns cos(angle) + sin(angle) as a complex number.
pub fn complex_cis(angle: impl Angle<Unitless = f64>) -> Complex<f64> {
    Complex::new(angle.cos(), angle.sin())
}

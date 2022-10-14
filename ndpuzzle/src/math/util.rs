//! Math utility functions.

use std::ops::{Add, Mul};

/// Linearly interpolates (unclamped) between two values.
pub fn mix<T>(a: T, b: T, t: f32) -> <T::Output as Add>::Output
where
    T: Mul<f32>,
    T::Output: Add,
{
    a * (1.0 - t) + b * t
}

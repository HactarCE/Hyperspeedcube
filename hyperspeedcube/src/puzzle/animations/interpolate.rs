//! Interpolation functions.

use std::f32::consts::PI;

/// Function that maps a float from the range 0.0 to 1.0 to another float
/// from 0.0 to 1.0.
pub type InterpolateFn = fn(f32) -> f32;

/// Interpolate using cosine from 0.0 to PI.
pub const COSINE: InterpolateFn = |x| (1.0 - (x * PI).cos()) / 2.0;

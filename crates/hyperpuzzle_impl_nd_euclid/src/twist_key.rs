use std::fmt;

use hypermath::collections::approx_hashmap::FloatHash;
use hypermath::{ApproxHashMapKey, Float, pga};
use hyperpuzzle_core::Axis;

/// Unique key for a twist.
#[derive(Debug, Clone)]
pub struct TwistKey {
    /// Axis that is twisted.
    axis: Axis,
    /// Transform to apply to pieces.
    ///
    /// This must be canonicalized first in order to work correctly. Construct
    /// using [`TwistKey::new()`] to avoid pitfalls.
    transform: pga::Motor,
}

impl ApproxHashMapKey for TwistKey {
    type Hash = (Axis, <pga::Motor as ApproxHashMapKey>::Hash);

    fn approx_hash(&self, float_hash_fn: impl FnMut(Float) -> FloatHash) -> Self::Hash {
        (self.axis, self.transform.approx_hash(float_hash_fn))
    }
}

impl TwistKey {
    /// Constructs a twist key from an axis and a transform, which does not need
    /// to be canonicalized.
    ///
    /// Returns `None` if `transform` could not be canonicalized.
    pub fn new(axis: Axis, transform: &pga::Motor) -> Option<TwistKey> {
        let transform = transform.canonicalize_up_to_180()?;
        Some(Self { axis, transform })
    }
}

impl fmt::Display for TwistKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { axis, transform } = self;
        write!(f, "twist with axis {axis} and transform {transform}")
    }
}

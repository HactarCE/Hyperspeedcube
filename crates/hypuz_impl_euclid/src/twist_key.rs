use std::fmt;
use std::hash::Hash;

use hypermath::{ApproxEq, ApproxHash, ApproxInternable, Precision, pga};
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

impl ApproxEq for TwistKey {
    fn approx_eq(&self, other: &Self, prec: Precision) -> bool {
        self.axis == other.axis && prec.eq(&self.transform, &other.transform)
    }
}

impl ApproxInternable for TwistKey {
    fn intern_floats<F: FnMut(&mut f64)>(&mut self, f: &mut F) {
        self.transform.intern_floats(f);
    }
}

impl ApproxHash for TwistKey {
    fn interned_eq(&self, other: &Self) -> bool {
        self.axis == other.axis && self.transform.interned_eq(&other.transform)
    }

    fn interned_hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.axis.hash(state);
        self.transform.interned_hash(state);
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

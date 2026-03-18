use hypuz_util::ti::TypedIndex;
use smallvec::SmallVec;

mod orbits;
mod solver;

use orbits::SubgroupOrbits;
pub use solver::ConstraintSolver;

/// Constraint on a group element based on how it acts on points.
///
/// An element `g` satisfies this constraint if `g * old = new`.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Constraint<P> {
    /// Original point.
    pub old: P,
    /// Transformed point.
    pub new: P,
}

impl<P: TypedIndex> From<[P; 2]> for Constraint<P> {
    fn from([old, new]: [P; 2]) -> Self {
        Self { old, new }
    }
}

/// Set of constraints on a group element based on how it acts on points.
///
/// This is used to specify a group element in a way that depends only on the
/// points (which can be assigned standard names), irrespective of the IDs
/// assigned to specific group elements.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct ConstraintSet<P> {
    /// List of constraints in arbitrary order.
    pub constraints: SmallVec<[Constraint<P>; 4]>,
}

impl<P: TypedIndex> From<&[[P; 2]]> for ConstraintSet<P> {
    fn from(pairs: &[[P; 2]]) -> Self {
        Self {
            constraints: pairs
                .iter()
                .map(|&[old, new]| Constraint { old, new })
                .collect(),
        }
    }
}

impl<P: TypedIndex, const N: usize> From<[[P; 2]; N]> for ConstraintSet<P> {
    fn from(value: [[P; 2]; N]) -> Self {
        Self::from(value.as_slice())
    }
}

impl<P: TypedIndex> FromIterator<Constraint<P>> for ConstraintSet<P> {
    fn from_iter<T: IntoIterator<Item = Constraint<P>>>(iter: T) -> Self {
        Self {
            constraints: iter.into_iter().collect(),
        }
    }
}

impl<P: TypedIndex> IntoIterator for ConstraintSet<P> {
    type Item = Constraint<P>;

    type IntoIter = smallvec::IntoIter<[Constraint<P>; 4]>;

    fn into_iter(self) -> Self::IntoIter {
        self.constraints.into_iter()
    }
}

impl<'a, P: TypedIndex> IntoIterator for &'a ConstraintSet<P> {
    type Item = Constraint<P>;

    type IntoIter = std::iter::Copied<std::slice::Iter<'a, Constraint<P>>>;

    fn into_iter(self) -> Self::IntoIter {
        self.constraints.iter().copied()
    }
}

impl<P: TypedIndex> ConstraintSet<P> {
    /// Empty constraint set.
    pub const EMPTY: Self = Self {
        constraints: SmallVec::new_const(),
    };

    /// Returns an iterator over the constraints.
    pub fn iter(&self) -> <&Self as IntoIterator>::IntoIter {
        self.constraints.iter().copied()
    }
}

#[cfg(test)]
mod tests;

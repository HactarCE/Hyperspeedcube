use smallvec::SmallVec;

mod orbits;
mod solver;

use orbits::SubgroupOrbits;
pub use solver::ConstraintSolver;

use crate::RefPoint;

/// Constraint on a group element based on how it acts on reference points.
///
/// An element `g` satisfies this constraint if `g * old = new`.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Constraint {
    /// Original point.
    pub old: RefPoint,
    /// Transformed point.
    pub new: RefPoint,
}

impl From<[RefPoint; 2]> for Constraint {
    fn from([old, new]: [RefPoint; 2]) -> Self {
        Self { old, new }
    }
}

/// Set of constraints on a group element based on how it acts on reference
/// points.
///
/// This is used to specify a group element in a way that depends only on the
/// reference points (which can be assigned standard names), irrespective of the
/// IDs assigned to specific group elements.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct ConstraintSet {
    /// List of constraints in arbitrary order.
    pub constraints: SmallVec<[Constraint; 4]>,
}

impl From<&[[RefPoint; 2]]> for ConstraintSet {
    fn from(pairs: &[[RefPoint; 2]]) -> Self {
        Self {
            constraints: pairs
                .iter()
                .map(|&[old, new]| Constraint { old, new })
                .collect(),
        }
    }
}

impl<const N: usize> From<[[RefPoint; 2]; N]> for ConstraintSet {
    fn from(value: [[RefPoint; 2]; N]) -> Self {
        Self::from(value.as_slice())
    }
}

impl FromIterator<Constraint> for ConstraintSet {
    fn from_iter<T: IntoIterator<Item = Constraint>>(iter: T) -> Self {
        Self {
            constraints: iter.into_iter().collect(),
        }
    }
}

impl IntoIterator for ConstraintSet {
    type Item = Constraint;

    type IntoIter = smallvec::IntoIter<[Constraint; 4]>;

    fn into_iter(self) -> Self::IntoIter {
        self.constraints.into_iter()
    }
}

impl<'a> IntoIterator for &'a ConstraintSet {
    type Item = Constraint;

    type IntoIter = std::iter::Copied<std::slice::Iter<'a, Constraint>>;

    fn into_iter(self) -> Self::IntoIter {
        self.constraints.iter().copied()
    }
}

impl ConstraintSet {
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

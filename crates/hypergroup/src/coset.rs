use std::collections::HashSet;

use smallvec::SmallVec;

use crate::{Group, GroupElementId};

/// [Subgroup] of a [`Group`].
///
/// [subgroup]: https://en.wikipedia.org/wiki/Subgroup
#[derive(Debug, Clone)]
pub struct Subgroup {
    /// Group that the subgroup belongs to.
    pub overgroup: Group,
    /// Number of elements in the subgroup.
    pub element_count: usize,
    /// Generating set for the subgroup.
    pub generators: SmallVec<[GroupElementId; 4]>,
}

impl Subgroup {
    /// Returns whether the subgroup is trivial (contains only the identity).
    ///
    /// This is equivalent to `subgroup.element_count == 1`.
    pub fn is_trivial(&self) -> bool {
        self.element_count == 1
    }

    /// Returns a list of the elements in the subgroup.
    pub fn elements(&self) -> Vec<GroupElementId> {
        let mut seen = HashSet::<GroupElementId>::from_iter([GroupElementId::IDENTITY]);
        crate::orbit_collect(GroupElementId::IDENTITY, &self.generators, |_, &e, &g| {
            let new_elem = self.overgroup.compose(e, g);
            seen.insert(new_elem).then_some(new_elem)
        })
    }

    /// Conjugates the subgroup `H` by an element `x`, returning `x H x^-1`.
    pub fn conjugate(&self, x: GroupElementId) -> Self {
        let overgroup = &self.overgroup;
        let x_inv = overgroup.inverse(x);
        self.map_generators(|g| overgroup.compose(overgroup.compose(x, g), x_inv))
    }

    /// Conjugates the subgroup `H` by the inverse of an element `x`, returning
    /// `x^-1 H x`.
    fn conjugate_inv(&self, x: GroupElementId) -> Self {
        let overgroup = &self.overgroup;
        let x_inv = overgroup.inverse(x);
        self.map_generators(|g| overgroup.compose(overgroup.compose(x_inv, g), x))
    }

    fn map_generators(&self, f: impl FnMut(GroupElementId) -> GroupElementId) -> Self {
        Self {
            overgroup: self.overgroup.clone(),
            element_count: self.element_count,
            generators: self.generators.iter().copied().map(f).collect(),
        }
    }
}

/// Conjugate [coset] of a [`Subgroup`] of a [`Group`].
///
/// More technically, this is a "conjugate coset," or a sandwich of a subgroup
/// by group elements: `l S r`. This itself is _not_ a coset of the
/// subgroup, but it can be written as any of the following:
///
/// - Left [coset] of a conjugated subgroup: `(l r) (r^-1 S r)`
/// - Right [coset] of a conjugated subgroup: `(l S l^-1) (l r)`
/// - Conjugate of a left [coset]: `r^-1 ((r l) S) r`
/// - Conjugate of a right [coset]: `l (S (r l)) l^-1`
///
/// This is **not** the same thing as a [double coset].
///
/// [coset]: https://en.wikipedia.org/wiki/Coset
/// [double coset]: https://en.wikipedia.org/wiki/Double_coset
#[derive(Debug, Clone)]
pub struct ConjugateCoset {
    /// Subgroup.
    pub subgroup: Subgroup,
    /// Element to multiply on the left of the subgroup.
    pub lhs: GroupElementId,
    /// Element to multiply on the right of the subgroup.
    pub rhs: GroupElementId,
}

impl ConjugateCoset {
    /// Converts the conjugate coset `l S r` to the left coset `(l r) (r^-1 S r)`.
    pub fn to_left_coset(&self) -> LeftCoset {
        LeftCoset {
            subgroup: self.subgroup.conjugate_inv(self.rhs),
            lhs: self.subgroup.overgroup.compose(self.lhs, self.rhs),
        }
    }
    /// Converts the conjugate coset `l S r` to the right coset `(l S l^-1) (l r)`.
    pub fn to_right_coset(&self) -> RightCoset {
        RightCoset {
            subgroup: self.subgroup.conjugate(self.lhs),
            rhs: self.subgroup.overgroup.compose(self.lhs, self.rhs),
        }
    }

    /// Returns a list of the elements in the coset.
    pub fn elements(&self) -> Vec<GroupElementId> {
        self.to_left_coset().elements()
    }
}

/// Left [coset].
///
/// [coset]: https://en.wikipedia.org/wiki/Coset
#[derive(Debug, Clone)]
pub struct LeftCoset {
    /// Subgroup.
    pub subgroup: Subgroup,
    /// Element to multiply on the left of the subgroup.
    pub lhs: GroupElementId,
}

impl LeftCoset {
    /// Returns a list of the elements in the coset.
    pub fn elements(&self) -> Vec<GroupElementId> {
        let mut seen = HashSet::<GroupElementId>::from_iter([self.lhs]);
        crate::orbit_collect(self.lhs, &self.subgroup.generators, |_, &e, &g| {
            let new_elem = self.subgroup.overgroup.compose(e, g);
            seen.insert(new_elem).then_some(new_elem)
        })
    }
}

/// Right [coset].
///
/// [coset]: https://en.wikipedia.org/wiki/Coset
#[derive(Debug, Clone)]
pub struct RightCoset {
    /// Subgroup.
    pub subgroup: Subgroup,
    /// Element to multiply on the right of the subgroup.
    pub rhs: GroupElementId,
}

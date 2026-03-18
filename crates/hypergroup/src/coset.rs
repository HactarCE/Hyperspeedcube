use hypuz_util::ti::TiSet;
use smallvec::SmallVec;

use crate::{Group, GroupElementId};

/// [Coset] of a [subgroup] of a [`Group`].
///
/// More technically, this is a "conjugate coset," or a sandwich of a subgroup
/// by group elements: `lhs * subgroup * rhs`. This itself is _not_ a coset of
/// the subgroup, but it can be written as any of the following:
///
/// - Left [coset] of a conjugated subgroup: `(lhs * rhs) * (rhs^-1 * subgroup *
///   rhs)`
/// - Right [coset] of a conjugated subgroup: `(lhs * subgroup * lhs^-1) * (lhs
///   * rhs)`
/// - Conjugate of a left [coset]: `rhs^-1 * ((rhs * lhs) * subgroup) * rhs`
/// - Conjugate of a right [coset]: `lhs * (subgroup * (rhs * lhs)) * lhs^-1`
///
/// Since a conjugated subgroup is still a subgroup, this type does in fact
/// represent a coset of some subgroup.
///
/// This is **not** the same thing as a [double coset].
///
/// [coset]: https://en.wikipedia.org/wiki/Coset
/// [subgroup]: https://en.wikipedia.org/wiki/Subgroup
/// [double coset]: https://en.wikipedia.org/wiki/Double_coset
pub struct Coset {
    /// Group that the subgroup belongs to.
    pub overgroup: Group,
    /// Number of elements in the subgroup.
    pub element_count: usize,
    /// Generating set for the subgroup.
    pub subgroup_generators: SmallVec<[GroupElementId; 4]>,

    /// Element to multiply on the left of the subgroup.
    pub lhs: GroupElementId,
}

impl Coset {
    /// Constructs a left coset from a "conjugate coset."
    ///
    /// A "conjugate coset" is a sandwich of a subgroup by group elements: `lhs
    /// * subgroup * rhs`. This itself is _not_ a coset of the subgroup, but it
    /// can be written as any of the following:
    ///
    /// - Left [coset] of a conjugated subgroup: `(lhs * rhs) * (rhs^-1 *
    ///   subgroup * rhs)`
    /// - Right [coset] of a conjugated subgroup: `(lhs * subgroup * lhs^-1) *
    ///   (lhs
    ///   * rhs)`
    /// - Conjugate of a left [coset]: `rhs^-1 * ((rhs * lhs) * subgroup) * rhs`
    /// - Conjugate of a right [coset]: `lhs * (subgroup * (rhs * lhs)) *
    ///   lhs^-1`
    ///
    /// This constructor uses the first construction.
    ///
    /// Note that a conjugated subgroup is in fact a subgroup. Also note that a
    /// conjugate coset is **not** the same thing as a [double coset].
    ///
    /// [coset]: https://en.wikipedia.org/wiki/Coset
    /// [subgroup]: https://en.wikipedia.org/wiki/Subgroup
    /// [double coset]: https://en.wikipedia.org/wiki/Double_coset
    pub fn from_conjugate_coset(
        overgroup: Group,
        element_count: usize,
        lhs: GroupElementId,
        subgroup_generators: impl IntoIterator<Item = GroupElementId>,
        rhs: GroupElementId,
    ) -> Self {
        let new_lhs = overgroup.compose(lhs, rhs);
        let rhs_inv = overgroup.inverse(rhs);
        let subgroup_generators = subgroup_generators
            .into_iter()
            .map(|g| overgroup.compose(overgroup.compose(rhs_inv, g), rhs))
            .collect();
        Self {
            overgroup,
            element_count,
            subgroup_generators,
            lhs: new_lhs,
        }
    }

    /// Returns the group that the subgroup belongs to.
    pub fn overgroup(&self) -> &Group {
        &self.overgroup
    }

    /// Returns whether the subgroup is trivial (contains only the identity).
    ///
    /// Note that if the subgroup is (and thus the coset) only contain one
    /// element, the one element of the coset might not be the identity.
    ///
    /// This is equivalent to `coset.element_count == 1`.
    pub fn is_trivial(&self) -> bool {
        self.element_count == 1
    }

    /// Returns the number of elements in the coset.
    pub fn element_count(&self) -> usize {
        self.element_count
    }

    /// Returns a list of the elements in the coset.
    pub fn elements(&self) -> Vec<GroupElementId> {
        let mut seen = TiSet::from_iter([self.lhs]); // Use `TiSet` to avoid allocation for small cosets
        crate::orbit_collect(self.lhs, &self.subgroup_generators, |_, &e, &g| {
            let new_elem = self.overgroup.compose(e, g);
            seen.insert(new_elem).then_some(new_elem)
        })
    }
}

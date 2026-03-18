use std::sync::Arc;

use crate::{Group, GroupElementId, ProductSubgroup};

/// Sandwich of a subgroup by group elements: `lhs * subgroup * rhs`.
///
/// This is _not_ a coset of the subgroup, but it can be written as any of the
/// following:
///
/// - Left [coset] of a conjugated subgroup: `(lhs * rhs) * (rhs^1 * subgroup *
///   rhs)`
/// - Right [coset] of a conjugated subgroup: `(lhs * subgroup * lhs^1) * (lhs *
///   rhs)`
/// - Conjugate of a left [coset]: `rhs^1 * ((rhs * lhs) * subgroup) * rhs`
/// - Conjugate of a right [coset]: `lhs * (subgroup * (rhs * lhs)) * lhs^1`
///
/// This is **not** the same thing as a [double coset].
///
/// [coset]: https://en.wikipedia.org/wiki/Coset
/// [double coset]: https://en.wikipedia.org/wiki/Double_coset
#[derive(Debug, Clone)]
pub struct ConjugateCoset {
    /// Element to multiply on the left of the subgroup.
    pub lhs: GroupElementId,
    /// Subgroup.
    pub subgroup: Arc<ProductSubgroup>,
    /// Element to multiple on the right of the subgroup.
    pub rhs: GroupElementId,
}

impl ConjugateCoset {
    /// Returns an arbitrary group element in the conjugate coset.
    pub fn arbitrary_element(&self) -> GroupElementId {
        self.overgroup().compose(self.lhs, self.rhs)
    }

    /// Returns the group that the subgroup is within.
    pub fn overgroup(&self) -> &Group {
        self.subgroup.overgroup()
    }
}

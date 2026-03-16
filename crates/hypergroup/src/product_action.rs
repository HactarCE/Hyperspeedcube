use std::sync::Arc;

use hypuz_util::ti::{TypedIndex, TypedIndexIter};

use super::*;

/// Group action of a product group acting on a the disjoint union of reference
/// points.
pub struct ProductGroupAction {
    /// Number of reference points that the product group acts on.
    reference_point_count: usize,
    /// Factor group actions.
    factors: PerFactorGroup<Arc<GroupAction>>,
    /// For each factor group: how much to add to its [`RefPoint`]s to get the
    /// corresponding [`RefPoint`]s in the product group.
    reference_point_offsets: PerFactorGroup<u16>,
    /// Product group that acts on the points.
    group: ProductGroup,
}

impl ProductGroupAction {
    pub fn new(factors: impl IntoIterator<Item = Arc<GroupAction>>) -> Self {
        let factors: PerFactorGroup<Arc<GroupAction>> = factors.into_iter().collect();

        let group = ProductGroup::new(
            factors
                .iter_values()
                .map(|action| Box::new(Arc::clone(&action.group)) as Box<dyn Send + Sync + Group>),
        );

        let reference_point_count = factors
            .iter_values()
            .map(|action| action.ref_points().len())
            .sum();

        let reference_point_offsets: PerFactorGroup<u16> = factors
            .iter_values()
            .scan(0, |offset, action| {
                let this_offset = *offset;
                *offset += action.reference_point_count as u16;
                Some(this_offset)
            })
            .collect();

        Self {
            reference_point_count,
            factors,
            reference_point_offsets,
            group,
        }
    }

    pub fn group(&self) -> &ProductGroup {
        &self.group
    }

    pub fn factors(&self) -> &PerFactorGroup<Arc<GroupAction>> {
        &self.factors
    }

    pub fn ref_points(&self) -> TypedIndexIter<RefPoint> {
        RefPoint::iter(self.reference_point_count)
    }

    pub fn ref_point_to_factor(&self, point: RefPoint) -> (FactorGroup, RefPoint) {
        let (factor, &offset) = self
            .reference_point_offsets
            .iter()
            .rfind(|&(_, &offset)| offset <= point.0)
            .expect("reference point index out of range");
        (factor, RefPoint(point.0 - offset))
    }

    pub fn try_ref_point_to_factor(
        &self,
        factor: FactorGroup,
        point: RefPoint,
    ) -> Option<RefPoint> {
        let i = point.0.checked_sub(self.reference_point_offsets[factor])?;
        ((i as usize) < self.factors[factor].reference_point_count).then_some(RefPoint(i))
    }

    pub fn ref_point_from_factor(&self, factor: FactorGroup, point: RefPoint) -> RefPoint {
        RefPoint(point.0 + self.reference_point_offsets[factor])
    }

    pub fn act(&self, element: GroupElementId, point: RefPoint) -> RefPoint {
        let (factor, old_point_in_factor) = self.ref_point_to_factor(point);
        let element_in_factor = self.group.element_in_factor(factor, element);
        let new_point_in_factor = self.factors[factor].act(element_in_factor, old_point_in_factor);
        RefPoint(new_point_in_factor.0 + self.reference_point_offsets[factor])
    }
}

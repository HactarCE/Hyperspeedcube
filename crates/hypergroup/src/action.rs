use std::{fmt, sync::Arc};

use hypuz_util::ti::{TypedIndex, TypedIndexIter};

use super::*;

/// Group action of a product group acting on a the disjoint union of reference
/// points.
///
/// This type is reference-counted and thus cheap to clone.
#[derive(Clone)]
pub struct GroupAction {
    /// Product group that acts on the points.
    group: Group,
    inner: Arc<GroupActionInner>,
}

struct GroupActionInner {
    /// Number of reference points that the product group acts on.
    ref_point_count: usize,
    /// Factor group actions.
    factors: PerFactorGroup<Arc<AbstractGroupActionLut>>,
    /// For each factor group: how much to add to its [`RefPoint`]s to get the
    /// corresponding [`RefPoint`]s in the product group.
    ref_point_offsets: PerFactorGroup<u16>,
}

impl fmt::Debug for GroupAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GroupAction")
            .field("ref_point_count", &self.inner.ref_point_count)
            .field("group", &self.group)
            .finish_non_exhaustive()
    }
}

impl GroupAction {
    pub(crate) fn from_factors(
        factors: impl IntoIterator<Item = Arc<AbstractGroupActionLut>>,
    ) -> GroupResult<Self> {
        let factors: PerFactorGroup<Arc<AbstractGroupActionLut>> = factors.into_iter().collect();

        let group = Group::from_factors(
            factors
                .iter_values()
                .map(|action| Arc::clone(action.group())),
        )?;

        let ref_point_count = factors
            .iter_values()
            .map(|action| action.ref_points().len())
            .sum();

        let ref_point_offsets: PerFactorGroup<u16> = factors
            .iter_values()
            .scan(0, |offset, action| {
                let this_offset = *offset;
                *offset += action.ref_point_count() as u16;
                Some(this_offset)
            })
            .collect();

        Ok(Self {
            group,
            inner: Arc::new(GroupActionInner {
                ref_point_count,
                factors,
                ref_point_offsets,
            }),
        })
    }

    /// Constructs a group action from a [direct product].
    ///
    /// [direct product]: https://en.wikipedia.org/wiki/Direct_product_of_groups
    pub fn product<'a>(factors: impl IntoIterator<Item = &'a Self>) -> GroupResult<Self> {
        Self::from_factors(
            factors
                .into_iter()
                .flat_map(|factor| factor.inner.factors.iter_values().cloned()),
        )
    }

    pub fn group(&self) -> &Group {
        &self.group
    }

    pub fn factors(&self) -> &PerFactorGroup<Arc<AbstractGroupActionLut>> {
        &self.inner.factors
    }

    pub fn ref_points(&self) -> TypedIndexIter<RefPoint> {
        RefPoint::iter(self.inner.ref_point_count)
    }

    pub(crate) fn ref_point_to_factor(&self, point: RefPoint) -> (FactorGroup, RefPoint) {
        let (factor, &offset) = self
            .inner
            .ref_point_offsets
            .iter()
            .rfind(|&(_, &offset)| offset <= point.0)
            .expect("reference point index out of range");
        (factor, RefPoint(point.0 - offset))
    }

    pub(crate) fn try_ref_point_to_factor(
        &self,
        factor: FactorGroup,
        point: RefPoint,
    ) -> Option<RefPoint> {
        let i = point.0.checked_sub(self.inner.ref_point_offsets[factor])?;
        ((i as usize) < self.factors()[factor].ref_point_count()).then_some(RefPoint(i))
    }

    pub fn ref_point_from_factor(&self, factor: FactorGroup, point: RefPoint) -> RefPoint {
        RefPoint(point.0 + self.inner.ref_point_offsets[factor])
    }

    pub fn act(&self, element: GroupElementId, point: RefPoint) -> RefPoint {
        let (factor, old_point_in_factor) = self.ref_point_to_factor(point);
        let element_in_factor = self.group.project_element_to_factor(factor, element);
        let new_point_in_factor =
            self.factors()[factor].act(element_in_factor, old_point_in_factor);
        RefPoint(new_point_in_factor.0 + self.inner.ref_point_offsets[factor])
    }
}

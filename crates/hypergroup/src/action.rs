use std::fmt;
use std::sync::Arc;

use hypuz_util::ti::{TypedIndex, TypedIndexIter};

use super::*;

/// Group action of a product group acting on a the disjoint union of points.
///
/// This type is reference-counted and thus cheap to clone.
#[derive(Clone)]
pub struct GroupAction<P> {
    /// Product group that acts on the points.
    group: Group,
    inner: Arc<GroupActionInner<P>>,
}

struct GroupActionInner<P> {
    /// Number of points that the product group acts on.
    point_count: usize,
    /// Factor group actions.
    factors: PerFactorGroup<Arc<AbstractGroupActionLut<P>>>,
    /// For each factor group: how much to add to its point indexes to get the
    /// corresponding point indexes in the product group.
    point_index_offsets: PerFactorGroup<usize>,
}

impl<P> fmt::Debug for GroupAction<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GroupAction")
            .field("point_count", &self.inner.point_count)
            .field("group", &self.group)
            .finish_non_exhaustive()
    }
}

impl<P: TypedIndex> GroupAction<P> {
    /// Constructs the trivial group action.
    pub fn trivial() -> Self {
        Self {
            group: Group::trivial(),
            inner: Arc::new(GroupActionInner {
                point_count: 0,
                factors: PerFactorGroup::new(),
                point_index_offsets: PerFactorGroup::new(),
            }),
        }
    }

    pub(crate) fn from_factors(
        factors: impl IntoIterator<Item = Arc<AbstractGroupActionLut<P>>>,
    ) -> GroupResult<Self> {
        let factors: PerFactorGroup<Arc<AbstractGroupActionLut<P>>> = factors.into_iter().collect();

        let group = Group::from_factors(
            factors
                .iter_values()
                .map(|action| Arc::clone(action.group())),
        )?;

        let point_count = factors
            .iter_values()
            .map(|action| action.points().len())
            .sum();

        let point_index_offsets: PerFactorGroup<usize> = factors
            .iter_values()
            .scan(0, |offset, action| {
                let this_offset = *offset;
                *offset += action.point_count();
                Some(this_offset)
            })
            .collect();

        Ok(Self {
            group,
            inner: Arc::new(GroupActionInner {
                point_count,
                factors,
                point_index_offsets,
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

    /// Returns the abstract group.
    pub fn group(&self) -> &Group {
        &self.group
    }

    pub(crate) fn factors(&self) -> &PerFactorGroup<Arc<AbstractGroupActionLut<P>>> {
        &self.inner.factors
    }

    /// Returns an iterator over the points acted on by the group.
    pub fn points(&self) -> TypedIndexIter<P> {
        P::iter(self.inner.point_count)
    }

    pub(crate) fn point_to_factor(&self, point: P) -> (FactorGroup, P) {
        let (factor, &offset) = self
            .inner
            .point_index_offsets
            .iter()
            .rfind(|&(_, &offset)| offset <= point.to_index())
            .expect("point index out of range");
        (
            factor,
            P::try_from_index(point.to_index() - offset).expect("error offsetting point index"),
        )
    }

    pub(crate) fn try_point_to_factor(&self, factor: FactorGroup, point: P) -> Option<P> {
        let i = point
            .to_index()
            .checked_sub(self.inner.point_index_offsets[factor])?;
        (i < self.factors()[factor].point_count()).then_some(P::try_from_index(i).ok()?)
    }

    pub(crate) fn point_from_factor(&self, factor: FactorGroup, point: P) -> P {
        P::try_from_index(point.to_index() + self.inner.point_index_offsets[factor])
            .expect("error offsetting point index")
    }

    /// Applies an action to a point.
    pub fn act(&self, element: GroupElementId, point: P) -> P {
        let (factor, old_point_in_factor) = self.point_to_factor(point);
        let element_in_factor = self.group.project_element_to_factor(factor, element);
        let new_point_in_factor =
            self.factors()[factor].act(element_in_factor, old_point_in_factor);
        P::try_from_index(new_point_in_factor.to_index() + self.inner.point_index_offsets[factor])
            .expect("error offsetting point index")
    }
}

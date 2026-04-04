use std::sync::Arc;

use hypuz_util::ti::{IndexOverflow, TypedIndex, TypedIndexIter};

use crate::{AbstractGroupLut, AbstractSubgroup, GeneratorId, GroupElementId};

/// Lookup table for an [action] of an abstract finite group (represented using
/// [`AbstractGroupLut`]) on a set of points (represented using the generic
/// parameter `P`, which must be implement [`TypedIndex`]).
///
/// The action does **not** need to be [faithful].
///
/// [action]: https://en.wikipedia.org/wiki/Group_action
/// [faithful]: https://mathworld.wolfram.com/FaithfulGroupAction.html
pub(crate) struct AbstractGroupActionLut<P> {
    /// Group that acts on the points.
    pub(super) group: Arc<AbstractGroupLut>,

    /// Number of points.
    pub(super) point_count: usize,

    /// Table containing the result of applying each generator to each point.
    pub(super) action_table: Vec<P>,
}

impl<P: TypedIndex> AbstractGroupActionLut<P> {
    /// Constructs a new group action lookup table from a function that composes
    /// a generator with a point. The number of points must be known ahead of
    /// time.
    pub fn from_fn(
        group: Arc<AbstractGroupLut>,
        point_count: usize,
        mut act: impl FnMut(GeneratorId, P) -> P,
    ) -> Result<Self, IndexOverflow> {
        let action_table =
            itertools::iproduct!(P::try_iter(point_count)?, group.generators().iter_keys())
                .map(|(p, g)| act(g, p))
                .collect();

        Ok(Self {
            group,
            point_count,
            action_table,
        })
    }

    /// Returns the group.
    pub fn group(&self) -> &Arc<AbstractGroupLut> {
        &self.group
    }

    /// Returns the number of points acted on by the group.
    pub fn point_count(&self) -> usize {
        self.point_count
    }
    /// Returns an iterator over all the points the group acts on.
    pub fn points(&self) -> TypedIndexIter<P> {
        P::iter(self.point_count)
    }

    /// Applies the action of a generator to a point.
    fn successor(&self, generator: GeneratorId, point: P) -> P {
        let index = point.to_index() * self.group.generators().len() + generator.0 as usize;
        self.action_table[index]
    }

    /// Applies the action of a group element to a point.
    pub fn act(&self, element: GroupElementId, point: P) -> P {
        self.group
            .factorization(element)
            .iter()
            .rfold(point, |p, &g| self.successor(g, p))
    }

    /// Returns the [pointwise stabilizer subgroup] of the group with respect to
    /// `fixed_points`. In other words: returns the subgroup containing exactly
    /// the elements that keep every point in `fixed_points` fixed.
    ///
    /// The returned subgroup always has the same generating set for the same
    /// subgroup, even if `fixed_points` is different. The generating set might
    /// not be minimal.
    ///
    /// In general, this algorithm takes approximately O(_nm_ + _ng_) time
    /// (where _n_ is the order of the group, _m_ is the number of fixed points,
    /// and _g_ is the resulting number of generators).
    ///
    /// [generating set]:
    ///     https://en.wikipedia.org/wiki/Generating_set_of_a_group
    /// [pointwise stabilizer subgroup]:
    ///     https://en.wikipedia.org/wiki/Group_action#Fixed_points_and_stabilizer_subgroups
    pub fn pointwise_stabilizer(&self, fixed_points: &[P]) -> AbstractSubgroup {
        let mut generators = vec![];
        let mut subgroup = AbstractSubgroup::new_trivial(Arc::clone(&self.group));

        for e in self.group.elements() {
            if !subgroup.contains(e) && fixed_points.iter().all(|&p| self.act(e, p) == p) {
                // The final subgroup generation takes longer than all smaller
                // subgroups combined because each subgroup is at least 2x
                // larger than the one before it. For this reason, we don't
                // bother trying to reuse previous results.
                generators.push(e);
                subgroup = AbstractSubgroup::new(Arc::clone(&self.group), generators.clone());
            }
        }

        subgroup
    }
}

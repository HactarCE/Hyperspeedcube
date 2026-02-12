use std::sync::Arc;

use hypuz_util::ti::{TiMask, TiVec, TypedIndex, TypedIndexIter};

use super::{AbstractGroup, Group, GroupElementId, PerGenerator, Subgroup};

hypuz_util::typed_index_struct! {
    /// Reference point acted on by a group.
    ///
    /// See [`GroupAction`].
    pub struct RefPoint(u16);
}

/// List containing a value per reference point.
pub type PerRefPoint<T> = TiVec<RefPoint, T>;

/// Group with an associated [action] on a set of [`Point`]s ("reference
/// points"), each assigned a [`RefPoint`] ID.
///
/// [action]: https://en.wikipedia.org/wiki/Group_action
pub struct GroupAction {
    pub(super) group: Arc<AbstractGroup>,

    /// Number of reference points.
    pub(super) reference_point_count: usize,

    /// Table containing the result of applying each generator to each reference
    /// point.
    pub(super) action_table: PerGenerator<PerRefPoint<RefPoint>>,
}

impl Group for GroupAction {
    fn group(&self) -> &AbstractGroup {
        &self.group
    }
}

impl GroupAction {
    /// Returns an iterator over the reference points.
    pub fn ref_points(&self) -> TypedIndexIter<RefPoint> {
        RefPoint::iter(self.reference_point_count)
    }

    /// Applies the action of a group element to a reference point.
    pub fn act(&self, element: GroupElementId, point: RefPoint) -> RefPoint {
        self.factorization(element)
            .iter()
            .rfold(point, |p, &g| self.action_table[g][p])
    }

    /// Returns the pointwise stabilizer of the given points.
    ///
    /// The pointwise stabilizer is the subgroup containing all elements that
    /// keep every point in `fixed_points` fixed.
    ///
    /// In general, this algorithm may take O(_nm_) time (where _n_ is the order
    /// of the group and _m_ is the number of fixed points).
    pub fn pointwise_stabilizer(&self, fixed_points: &[RefPoint]) -> Subgroup {
        let mut subgroup = Subgroup::new_trivial(Arc::clone(&self.group));
        let mut unprocessed_elements = TiMask::new_full(self.element_count());
        unprocessed_elements.remove(GroupElementId::IDENTITY);
        while let Some(e) = unprocessed_elements.pop_first() {
            let preserves_fixed_points = fixed_points
                .iter()
                .all(|&ref_point| self.act(e, ref_point) == ref_point);
            if preserves_fixed_points {
                subgroup.add_generators(&[e]);
                continue;
            }
        }
        subgroup
    }

    /// Returns the orbits of the reference points under a group action. See
    /// [`SubgroupOrbits`].
    pub(super) fn orbits(&self, subgroup: &Subgroup) -> SubgroupOrbits {
        let mut deorbiters = PerRefPoint::from_iter(self.ref_points().map(|p| RefPointDeorbiter {
            orbit_representative: p,
            deorbiter: GroupElementId::IDENTITY,
        }));
        let mut points_seen = TiMask::new_empty(self.reference_point_count);
        for init in self.ref_points() {
            if !points_seen.contains(init) {
                points_seen.insert(init); // representative is self, deorbited is identity
                super::orbit(init, subgroup.generating_set(), |&point, &g| {
                    let new_point = self.act(g, point);
                    (!points_seen.contains(new_point)).then(|| {
                        points_seen.insert(new_point);
                        deorbiters[new_point] = RefPointDeorbiter {
                            orbit_representative: init,
                            deorbiter: self.compose(self.inverse(g), deorbiters[point].deorbiter),
                        };
                        new_point
                    })
                });
            }
        }
        SubgroupOrbits { deorbiters }
    }
}

/// Orbits of reference points in a subgroup.
///
/// Each orbit of points is assigned a representative point _p_, and each point
/// _q_ in the orbit of _p_ is assigned a element _g_ from the subgroup such
/// that _g p = q_. We call _g_ the **deorbiter** of _q_.
pub(super) struct SubgroupOrbits {
    pub(super) deorbiters: PerRefPoint<RefPointDeorbiter>,
}

/// Orbit decomposition of a reference point.
///
/// `deorbiter * this_reference_point = orbit_representative`
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(super) struct RefPointDeorbiter {
    /// Representative reference point for the entire orbit.
    ///
    /// This serves as a unique identifier for the orbit.
    pub(super) orbit_representative: RefPoint,
    /// Transform from this reference point to `orbit_representative`.
    pub(super) deorbiter: GroupElementId,
}

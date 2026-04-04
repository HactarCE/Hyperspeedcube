use std::collections::HashMap;
use std::sync::Arc;

use hypuz_util::ti::{TiMask, TiVec, TypedIndex};

use crate::{AbstractGroupActionLut, AbstractSubgroup, GroupElementId};

/// Orbits of points in a subgroup of an [`AbstractGroupActionLut`]. The
/// subgroup is represented as an [`AbstractSubgroup`].
///
/// Each point `p` is given an **orbit decomposition**, which is a combination
/// of a group element `deorbiter` in the subgroup and a point
/// `orbit_representative` such that `deorbiter * p = orbit_representative`.
/// `orbit_representative` is unique within an orbit.
pub(super) struct SubgroupOrbits<P> {
    /// Subgroup.
    pub subgroup: Arc<AbstractSubgroup>,

    /// Orbit representative for each point.
    ///
    /// All points in the same orbit share exactly one orbit representative,
    /// which is chosen deterministically.
    pub orbit_representatives: TiVec<P, P>,
    /// Deorbiter for each point `p` such that `deorbiters[p] * p =
    /// orbit_representatives[p]`.
    ///
    /// There may be multiple valid deorbiters; one is chosen arbitrarily.
    pub deorbiters: TiVec<P, GroupElementId>,

    /// List of points in the largest orbit. When multiple orbits are largest,
    /// one is chosen deterministically.
    ///
    /// If all orbits are trivial (containing only a single element) then this
    /// is empty.
    pub canonical_largest_orbit: Vec<P>,
}

impl<P: TypedIndex> SubgroupOrbits<P> {
    /// Constructs the total subgroup that contains the entire group.
    pub fn new_total(action: &AbstractGroupActionLut<P>) -> Self {
        let subgroup = Arc::new(AbstractSubgroup::new_total(Arc::clone(&action.group())));
        Self::new(action, subgroup)
    }

    /// Constructs a stabilizer subgroup from a generating set.
    pub fn new(action: &AbstractGroupActionLut<P>, subgroup: Arc<AbstractSubgroup>) -> Self {
        let group = action.group();

        debug_assert!(Arc::ptr_eq(action.group(), subgroup.overgroup()));

        // Compute deorbiters.
        let mut orbit_representatives = TiVec::<P, P>::from_iter(action.points());
        let mut deorbiters = TiVec::<P, GroupElementId>::new_with_len(action.point_count());
        let mut points_seen = TiMask::<P>::new_empty(action.point_count());
        for init in action.points() {
            if !points_seen.contains(init) {
                // `init` is the first point visited in its orbit, so it is the
                // canonical representative of its orbit. Its deorbiter is the
                // identity.
                points_seen.insert(init);

                crate::orbit(init, subgroup.generators(), |&point, &g| {
                    let new_point = action.act(g, point);
                    (!points_seen.contains(new_point)).then(|| {
                        points_seen.insert(new_point);
                        orbit_representatives[new_point] = init;
                        deorbiters[new_point] = group.compose(deorbiters[point], group.inverse(g));
                        new_point
                    })
                });
            }
        }

        // Check the invariant that `deorbiters[p] * p = orbit_representatives[p]`.
        #[cfg(debug_assertions)]
        for p in action.points() {
            assert_eq!(action.act(deorbiters[p], p), orbit_representatives[p]);
        }

        // Select canonical largest orbit.
        // IIFE to mimic try_block
        let canonical_largest_orbit = (|| {
            let mut orbit_sizes: HashMap<P, usize> = HashMap::new();
            for &orbit_representative in orbit_representatives.iter_values() {
                *orbit_sizes.entry(orbit_representative).or_default() += 1;
            }
            let size_of_largest_orbit = *orbit_sizes.values().max()?;
            if size_of_largest_orbit <= 1 {
                return None;
            }
            let canonical_representative =
                orbit_representatives.find(|_, repr| orbit_sizes[repr] == size_of_largest_orbit)?;
            Some(
                orbit_representatives
                    .iter_filter(|_, &repr| repr == canonical_representative)
                    .collect(),
            )
        })()
        .unwrap_or(vec![]);

        SubgroupOrbits {
            subgroup,
            orbit_representatives,
            deorbiters,
            canonical_largest_orbit,
        }
    }
}

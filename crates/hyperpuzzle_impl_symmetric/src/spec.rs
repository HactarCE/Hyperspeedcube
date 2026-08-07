use std::sync::Arc;

use eyre::{Result, bail, ensure};
use hypergroup::{AbbrGenSeq, CoxeterMatrix, GroupAction, GroupElementId, IsometryGroup};
use hypermath::{Subspace, prelude::*};
use hyperpuzzle_core::CatalogId;
use hypuz_notation::Str;
use itertools::Itertools;

use crate::builder::TwistSystemProduct;
use crate::{CutDistances, NamedPoint, PerNamedPoint};

/// Specification for a twist system factor.
pub struct FactorTwistSystemSpec {
    /// ID for the twist system.
    pub id: CatalogId,
    /// Number of dimensions.
    pub ndim: u8,
    /// Symmetry for the twist system.
    pub coxeter_matrix: Option<CoxeterMatrix>,
    /// Orbits of axes.
    pub axis_orbits: Vec<SimpleOrbitSpec>,
    /// Orbits of named points, which are used to name axes and twists.
    pub named_point_orbits: Vec<NamedPointOrbitSpec>,
    /// Orbits of named point sets, which are used to define stabilizer twists
    /// in higher dimensions.
    pub named_point_set_orbits: Vec<NamedPointSetOrbitSpec>,
    /// Orbits of stabilizer twists.
    pub stabilizer_twist_orbits: Vec<StabilizerTwistOrbitSpec>,
}

/// Specification for an orbit of named point sets in a
/// [`FactorTwistSystemSpec`].
///
/// This struct only contains info about one named point set in the orbit; the
/// orbit is generated using the grip group.
pub struct NamedPointSetOrbitSpec {
    /// Names of the named points in the set.
    pub named_points: Vec<Str>,
    /// Gizmo pole distance for stabilizer twists using the named point set.
    pub gizmo_pole_distance: Float,
}

/// Specification for an orbit of twists in a [`FactorTwistSystemSpec`].
///
/// This struct only contains info about one twist in the orbit; the orbit is
/// generated using the grip group.
pub struct StabilizerTwistOrbitSpec {
    /// Name of the axis to twist.
    pub axis_name: Str,
    /// Names of the named points which remain stabilized by the axis.
    pub named_points: Vec<Str>,
    /// Gizmo pole distance for the twist.
    pub gizmo_pole_distance: Float,
}

/// Specification for a factor of a [`ProductPuzzleSpec`].
#[derive(Debug)]
pub struct FactorPuzzleSpec {
    /// ID for the puzzle.
    pub id: CatalogId,
    /// Name for the puzzle.
    pub name: String,

    /// Symmetry for the puzzle factor.
    // TODO: split axes symmetry and facets symmetry (requires expanding shape
    // symmetry before slicing)
    pub coxeter_matrix: CoxeterMatrix,

    /// Named points, which are used to name facets.
    pub named_point_orbits: Vec<NamedPointOrbitSpec>,
    /// Orbits of facets, identified by their facet pole.
    ///
    /// Each facet is assigned a unique color.
    pub facet_orbits: Vec<SimpleOrbitSpec>,

    /// ID for the color system, or `None` to use an ad-hoc color system.
    pub colors_id: Option<CatalogId>,
    /// Twist system.
    pub twists: Arc<TwistSystemProduct>,

    /// Cut distances for each axis orbit.
    pub axis_orbit_cut_distances: Vec<CutDistances>,
}

/// Specification for an orbit of named points in a [`FactorTwistSystemSpec`].
#[derive(Debug, Clone)]
pub struct NamedPointOrbitSpec {
    /// Vector, name, and generator sequence for named point in the orbit.
    pub orbit_members: Vec<NamedPointSpec>,
}

impl NamedPointOrbitSpec {
    /// Returns the lexicographically first name in the orbit, or an empty
    /// string if the orbit is empty.
    pub fn min_name(&self) -> &str {
        self.orbit_members
            .iter()
            .map(|point| point.name.as_str())
            .min()
            .unwrap_or("")
    }
}

impl NamedPointOrbitSpec {
    /// Returns the number of named points in the orbit.
    #[allow(clippy::len_without_is_empty)] // should never be empty
    pub fn len(&self) -> usize {
        self.orbit_members.len()
    }
}

/// Specification for a named point in a [`NamedPointOrbitSpec`].
#[derive(Debug, Clone)]
pub struct NamedPointSpec {
    /// Vector corresponding to the member.
    pub vector: Vector,
    /// Name of the member.
    pub name: Str,
    /// Abbreviated generator sequence, which is used for the HPS code generator
    /// utility.
    pub abbr_gen_seq: AbbrGenSeq,
}

/// Specification for an orbit of axes or facets in a [`FactorTwistSystemSpec`]
/// or [`FactorPuzzleSpec`] respectively.
///
/// This does not include the full name because those will be automatically
/// generated based on the vector and its relation to nearby named points.
#[derive(Debug, Clone)]
pub struct SimpleOrbitSpec {
    /// Prefix for the axis orbit.
    pub prefix: Str,
    /// Vector for one axis in the orbit.
    pub vector: Vector,
}

impl SimpleOrbitSpec {
    /// Expands the orbit, returning for each element:
    ///
    /// - a group element that transforms the first element of the orbit to this
    ///   one
    /// - the vector
    /// - the canonical name, in terms of named points
    ///
    /// The named point locations **must** be normalized.
    pub(crate) fn expand_and_name(
        &self,
        group: &IsometryGroup,
        named_point_unit_vectors: &PerNamedPoint<Vector>,
        named_point_action: &GroupAction<NamedPoint>,
    ) -> Result<Vec<(GroupElementId, Vector, Vec<Vec<NamedPoint>>)>> {
        let orbit = group.orbit_geometric(self.vector.clone());

        let Some(unit_init_vector) = self.vector.normalize() else {
            // No named points needed! Hopefully the orbit has a prefix;
            // otherwise the name will be empty.
            return Ok(vec![(
                GroupElementId::IDENTITY,
                self.vector.clone(),
                vec![],
            )]);
        };
        let init_vector = unit_init_vector.clone();

        if orbit.len() == 1 {
            // No named points needed! Same as above
            return Ok(vec![(
                GroupElementId::IDENTITY,
                self.vector.clone(),
                vec![],
            )]);
        }

        // Name each member based on the nearest named points.
        let mut member_names: Vec<Vec<Vec<NamedPoint>>> = orbit.iter().map(|_| vec![]).collect();
        let distances: PerNamedPoint<Float> =
            named_point_unit_vectors.map_ref(|_, loc| (&init_vector - loc).mag2());

        let mut last_multiplicity = member_names.len(); // all names are the same at the start
        let mut candidates: Vec<NamedPoint> = named_point_unit_vectors.iter_keys().collect_vec();
        let mut subspace = Subspace::new();
        loop {
            // Select all the named points that are equally closest to the
            // member.
            let Some(min_distance) = candidates
                .iter()
                .map(|&p| distances[p])
                .min_by(|a, b| APPROX.cmp(a, b))
            else {
                bail!("named points do not span the space");
            };
            let closest_points = candidates
                .iter()
                .copied()
                .filter(|&p| APPROX.eq(distances[p], min_distance))
                .collect_vec();

            // Append that set of named points to the name for each member.
            for (&(elem, _), member_name) in std::iter::zip(&orbit, &mut member_names) {
                member_name.push(
                    closest_points
                        .iter()
                        .map(|&p| named_point_action.act(elem, p))
                        .sorted()
                        .collect(),
                );
            }

            // Check the "multiplicity" of each name; i.e., how many axes have the
            // same name. This should be the same for all names.
            let first_member_name = &member_names[0];
            let multiplicity = member_names
                .iter()
                .filter(|&name| name == first_member_name)
                .count();
            // Sanity-check that all names have the same multiplicity.
            if !member_names
                .iter()
                .counts()
                .into_iter()
                .all(|(_, count)| count == multiplicity)
            {
                bail!("names have different multiplicies in expand_and_name()");
            }
            // If the names are all unique, then we're done!
            if multiplicity == 1 {
                // Return the vector and the name.
                return Ok(std::iter::zip(orbit, member_names)
                    .map(|((elem, vector), name)| (elem, vector, name))
                    .collect());
            }
            // If the multiplicity hasn't changed, then this iteration was useless
            // so undo it. I'm not sure whether this case is possible.
            if multiplicity == last_multiplicity {
                for name in &mut member_names {
                    name.pop();
                }
            }
            last_multiplicity = multiplicity;

            // Remove from consideration all named points within the subspace,
            // because they will not help us narrow down the member name.
            let old_ndim = subspace.ndim();
            for p in closest_points {
                subspace.add(&named_point_unit_vectors[p]);
            }
            ensure!(
                subspace.ndim() > old_ndim,
                "expand_and_name() failed to make progress",
            );
            candidates.retain(|&p| !subspace.contains(&named_point_unit_vectors[p]));
        }
    }
}

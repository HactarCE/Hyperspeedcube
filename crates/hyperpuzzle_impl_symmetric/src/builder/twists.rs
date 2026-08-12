use std::num::NonZeroI32;
use std::sync::Arc;

use eyre::{Context, OptionExt, Result, bail, eyre};
use hypergroup::{
    ConjugateCoset, GroupAction, GroupElementId, IsometryGroup, SubgroupAction,
    SubgroupConstraintSolver,
};
use hypermath::{Float, Matrix, Point, Vector, VectorRef};
use hyperpuzzle_core::catalog::BuildCtx;
use hyperpuzzle_core::{
    Axis, AxisSystem, CatalogId, CatalogObject, ComponentList, IndexOverflow, Names, PerAxis,
    TwistSystem, TypedIndex, TypedIndexIter,
};
use hyperpuzzle_impl_nd_euclid::NdEuclidAxisVectors;
use hypuz_notation::family::SequentialLowercaseName;
use hypuz_util::{FloatMinMaxByIteratorExt, FloatMinMaxIteratorExt};
use itertools::Itertools;
use parking_lot::Mutex;
use smallvec::smallvec;

use super::{FactorNamedPointBasedNames, NamedPointOrbit, ProductNamedPointBasedNames};
use crate::{
    FactorTwistSystemSpec, NamedPoint, NamedPointSet, PerNamedPoint, StabilizerFamily,
    SymmetricTwistSystemAxisOrbit, SymmetricTwistSystemComponent, UniqueMinimalClockwiseGenerator,
};

#[derive(Debug, Clone)]
pub struct TwistSystemFactor {
    pub id: CatalogId,
    pub name: String,
    pub names: FactorNamedPointBasedNames<Axis>,
    pub axis_orbits: Vec<AxisOrbit>,
    pub named_point_orbits: Vec<NamedPointOrbit>,
}

impl TwistSystemFactor {
    #[must_use]
    fn offset_ids_by(
        &self,
        axis_id_offset: usize,
        named_point_id_offset: usize,
    ) -> Result<Self, IndexOverflow> {
        Ok(Self {
            id: self.id.clone(),
            name: self.name.clone(),
            names: self.names.clone(),
            axis_orbits: self
                .axis_orbits
                .iter()
                .map(|orbit| orbit.offset_ids_by(axis_id_offset, named_point_id_offset))
                .try_collect()?,
            named_point_orbits: self
                .named_point_orbits
                .iter()
                .map(|orbit| orbit.offset_ids_by(named_point_id_offset))
                .try_collect()?,
        })
    }

    fn update_stabilizer_twists(&mut self, total_ndim: u8, new_sets: &[(NamedPointSet, Float)]) {
        for orbit in &mut self.axis_orbits {
            if total_ndim <= 4 {
                orbit.stabilizer_twists.extend_from_slice(new_sets);
            } else {
                orbit.stabilizer_twists.clear();
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct AxisOrbit {
    /// Number of axes in the orbit.
    pub len: usize,
    /// ID offset of the axes in the orbit.
    ///
    /// IDs within an orbit always count starting from 0, but the puzzle may
    /// have multiple sets and so puzzle-facing IDs for axes must start counting
    /// from this offset.
    pub id_offset: usize,
    /// Orbits of nontrivial stabilizer twist families, along with their gizmo
    /// pole distances.
    ///
    /// Here, "nontrivial" means that the named point set is nonempty. E.g., the
    /// named point set is typically empty for twists on rotational 3D puzzles,
    /// but these do not need to be tracked. But in rotational 4D puzzles, they
    /// do need to be tracked.
    ///
    /// Because they are not used in higher dimensions, this list is made empty
    /// in 5D+.
    pub stabilizer_twists: Vec<(NamedPointSet, Float)>,
}

impl AxisOrbit {
    pub fn first(&self) -> Axis {
        Axis(self.id_offset as _) // already checked at construction
    }

    pub fn axes(&self) -> TypedIndexIter<Axis> {
        Axis::iter_range(self.id_offset..self.id_offset + self.len) // already checked at constuction
    }

    fn offset_ids_by(
        &self,
        axis_id_offset: usize,
        named_point_id_offset: usize,
    ) -> Result<Self, IndexOverflow> {
        let new_id_offset = axis_id_offset + self.id_offset;
        Axis::try_iter_range(new_id_offset..new_id_offset + self.len)?; // check for overflow
        Ok(Self {
            len: self.len,
            id_offset: new_id_offset,
            stabilizer_twists: self
                .stabilizer_twists
                .iter()
                .map(|(set, distance)| Ok((set.offset_ids_by(named_point_id_offset)?, *distance)))
                .try_collect()?,
        })
    }
}

/// Twist system of a puzzle under construction.
#[derive(Debug, Clone)]
pub struct TwistSystemProduct {
    /// ID computed from `factor_ids`.
    pub id: CatalogId,

    /// Grip group.
    pub group: IsometryGroup,
    /// Mirror planes from the Coxeter matrix for the grip group.
    pub coxeter_mirrors: Vec<Vector>,

    /// Action of the grip group on named points.
    pub named_point_action: GroupAction<NamedPoint>,
    /// Vector for each named point.
    pub named_point_vectors: PerNamedPoint<Vector>,
    /// Normalized vector for each named point.
    pub named_point_unit_vectors: PerNamedPoint<Vector>,

    /// Action of the grip group on axes.
    pub axis_action: GroupAction<Axis>,
    /// Vector for each axis.
    ///
    /// The vector is not necessarily normalized. Its magnitude determines the
    /// placement of twist gizmos in 3D and 4D. For a facet-turning puzzle, each
    /// axis vector will typically be scaled to match the distance of its
    /// corresponding facet.
    pub axis_vectors: PerAxis<Vector>,

    pub factors: Vec<TwistSystemFactor>,

    /// Nonempty sets of named points, each with a gizmo pole distance. Each
    /// orbit has only one representative in this list.
    ///
    /// Generally, every named point orbit should have a set in this list
    /// containing one named point from its orbit.
    ///
    /// These named point sets are used to construct twist gizmos and stabilizer
    /// twists for 4D puzzles. Because they are not needed in higher dimensions,
    /// this list is made empty in 4D+.
    pub named_point_set_orbits: Vec<(NamedPointSet, Float)>,
}

impl CatalogObject for TwistSystemProduct {
    fn catalog_type_name() -> &'static str {
        "symmetric twist system product"
    }

    fn id(&self) -> &CatalogId {
        &self.id
    }
}

impl TwistSystemProduct {
    /// Returns the number of the dimensions of the puzzle.
    pub fn ndim(&self) -> u8 {
        self.group.ndim()
    }

    /// Constructs the empty axis system, which is the identity of the direct
    /// product.
    pub fn direct_product_identity() -> Self {
        Self {
            id: crate::product_id([].into_iter()),
            group: IsometryGroup::trivial(),
            coxeter_mirrors: vec![],
            named_point_action: GroupAction::trivial(),
            named_point_vectors: PerNamedPoint::new(),
            named_point_unit_vectors: PerNamedPoint::new(),
            axis_action: GroupAction::trivial(),
            axis_vectors: PerAxis::new(),
            factors: vec![],
            named_point_set_orbits: vec![],
        }
    }

    pub fn new_empty(ndim: u8) -> Self {
        Self {
            id: CatalogId::new(
                "empty".parse().expect("bad Id"),
                [(ndim as i64).into()],
                None,
            ),
            ..Self::direct_product_identity()
        }
    }

    /// Constructs a product twist system builder with a single factor.
    pub fn new_factor(
        spec: &FactorTwistSystemSpec,
        warn_fn: &mut impl FnMut(eyre::Report),
    ) -> Result<Self> {
        let coxeter_mirrors;
        let group;
        if let Some(coxeter_matrix) = &spec.coxeter_matrix {
            if coxeter_matrix.generator_count() != spec.ndim {
                bail!(
                    "ndim={}, but coxeter_matrix_ndim={}",
                    spec.ndim,
                    coxeter_matrix.generator_count(),
                );
            }
            coxeter_mirrors = coxeter_matrix
                .mirrors()?
                .cols()
                .map(|col| col.to_vector())
                .collect_vec();
            let unshuffled_group = coxeter_matrix
                .isometry_group()
                .wrap_err("error expanding twist symmetries")?;

            // TODO: shuffle group generators to improve average word length,
            //       but make sure to not cause float precision issues
            group = unshuffled_group;

            // // Shuffle group generators to improve average word length, making some
            // // group operations faster.
            // group = crate::shuffle_group_generators(&unshuffled_group, &mut rand::rng())
            //     .wrap_err("error shuffling twist symmetry generators")?;
        } else {
            coxeter_mirrors = vec![];
            group = IsometryGroup::trivial_with_ndim(spec.ndim);
        }

        let (named_point_vectors, named_point_unit_vectors, named_point_orbits, mut names) =
            FactorNamedPointBasedNames::<Axis>::from_spec(&group, &spec.named_point_orbits)?;

        let named_point_points = named_point_vectors.map_ref(|_, v| Point(v.clone()));
        let named_point_action = group.action_on_points(&named_point_points)?;

        let mut axis_orbits = vec![];
        let mut axis_deorbiters = PerAxis::new();
        let mut axis_which_orbit = PerAxis::new();
        let mut axis_vectors = PerAxis::new();
        let mut axis_id_offset = 0;
        for (orbit_index, orbit) in spec.axis_orbits.iter().enumerate() {
            let orbit_members =
                orbit.expand_and_name(&group, &named_point_unit_vectors, &named_point_action)?;
            axis_orbits.push(AxisOrbit {
                len: orbit_members.len(),
                id_offset: axis_id_offset,
                stabilizer_twists: vec![], // will be populated later
            });
            axis_id_offset += orbit_members.len();
            for (undeorbiter, axis_vector, axis_name) in orbit_members {
                axis_deorbiters.push(group.inverse(undeorbiter))?;
                axis_which_orbit.push(orbit_index)?;
                axis_vectors.push(axis_vector)?;
                names.add_member(&orbit.prefix, axis_name)?;
            }
        }

        let axis_points = axis_vectors.map_ref(|_, v| Point(v.clone()));
        let axis_action = group.action_on_points(&axis_points)?;

        // Populate stabilizer twists
        for orbit in &spec.stabilizer_twist_orbits {
            let axis = names.member_from_name(&orbit.axis_name)?;
            let stabilized_points = NamedPointSet::new(
                orbit
                    .named_points
                    .iter()
                    .map(|s| names.named_point_from_name(s))
                    .try_collect()?,
            )?;
            let axis_orbit_index = axis_which_orbit[axis];
            let deorbiter = axis_deorbiters[axis];
            axis_orbits[axis_orbit_index].stabilizer_twists.push((
                stabilized_points.transform_by_group_element(&named_point_action, deorbiter),
                orbit.gizmo_pole_distance,
            ));
        }

        let mut named_point_set_orbits: Vec<(NamedPointSet, Float)> = vec![];
        // Add a singleton named point set for each named point.
        for orbit in &named_point_orbits {
            named_point_set_orbits.push((
                NamedPointSet::new(smallvec![orbit.first()?])?,
                named_point_vectors[orbit.first()?].mag(),
            ));
        }
        // Add non-singleton named point sets.
        for orbit in &spec.named_point_set_orbits {
            let points = orbit
                .named_points
                .iter()
                .map(|s| names.named_point_from_name(s))
                .try_collect()?;
            named_point_set_orbits.push((NamedPointSet::new(points)?, orbit.gizmo_pole_distance));
        }

        let id = crate::product_id([&spec.id].into_iter());
        let factor = TwistSystemFactor {
            id: spec.id.clone(),
            name: spec.name.clone(),
            names,
            axis_orbits,
            named_point_orbits,
        };

        Ok(Self {
            id,

            group,
            coxeter_mirrors,

            named_point_action,
            named_point_vectors,
            named_point_unit_vectors,

            axis_action,
            axis_vectors,

            factors: vec![factor],

            named_point_set_orbits,
        })
    }

    /// Returns the direct product of two axis systems.
    ///
    /// See [`super::ProductPuzzleBuilder::direct_product()`].
    pub fn direct_product(&self, rhs: &Self) -> Result<Self> {
        let a = self;
        let b = rhs;
        let ndim = a.ndim() + b.ndim();

        if (a.len() + b.len()).saturating_sub(1) > Axis::MAX_INDEX {
            return Err(IndexOverflow::new::<Axis>().into());
        }

        let group = IsometryGroup::product([&a.group, &b.group])?;
        let named_point_action =
            GroupAction::product([&a.named_point_action, &b.named_point_action])?;
        let axis_action = GroupAction::product([&a.axis_action, &b.axis_action])?;

        let coxeter_mirrors = std::iter::chain(
            a.coxeter_mirrors
                .iter()
                .map(|v| crate::lift_vector_by_ndim(v, 0, a.ndim(), b.ndim())),
            b.coxeter_mirrors
                .iter()
                .map(|v| crate::lift_vector_by_ndim(v, a.ndim(), b.ndim(), 0)),
        )
        .collect();

        let named_point_vectors = std::iter::chain(
            a.named_point_vectors
                .iter_values()
                .map(|v| crate::lift_vector_by_ndim(v, 0, a.ndim(), b.ndim())),
            b.named_point_vectors
                .iter_values()
                .map(|v| crate::lift_vector_by_ndim(v, a.ndim(), b.ndim(), 0)),
        )
        .collect();
        let named_point_unit_vectors = std::iter::chain(
            a.named_point_unit_vectors
                .iter_values()
                .map(|v| crate::lift_vector_by_ndim(v, 0, a.ndim(), b.ndim())),
            b.named_point_unit_vectors
                .iter_values()
                .map(|v| crate::lift_vector_by_ndim(v, a.ndim(), b.ndim(), 0)),
        )
        .collect();
        let axis_vectors = std::iter::chain(
            a.axis_vectors
                .iter_values()
                .map(|v| crate::lift_vector_by_ndim(v, 0, a.ndim(), b.ndim())),
            b.axis_vectors
                .iter_values()
                .map(|v| crate::lift_vector_by_ndim(v, a.ndim(), b.ndim(), 0)),
        )
        .collect();

        let lift_b_point_set =
            |points: &NamedPointSet| points.offset_ids_by(a.named_points_count());

        let a_new_named_point_set_orbits: Vec<(NamedPointSet, f64)> =
            a.named_point_set_orbits.clone();
        let b_new_named_point_set_orbits: Vec<(NamedPointSet, f64)> = b
            .named_point_set_orbits
            .iter()
            .map(|(set, distance)| eyre::Ok((lift_b_point_set(set)?, *distance)))
            .try_collect()?;

        let mut a_new_factors: Vec<TwistSystemFactor> = a.factors.clone();
        let mut b_new_factors: Vec<TwistSystemFactor> = b
            .factors
            .iter()
            .map(|factor| factor.offset_ids_by(a.axis_vectors.len(), a.named_point_vectors.len()))
            .try_collect()?;
        for factor in &mut a_new_factors {
            factor.update_stabilizer_twists(ndim, &b_new_named_point_set_orbits);
        }
        for factor in &mut b_new_factors {
            factor.update_stabilizer_twists(ndim, &a_new_named_point_set_orbits);
        }

        let factors: Vec<TwistSystemFactor> =
            std::iter::chain(a_new_factors, b_new_factors).collect();

        let named_point_set_orbits = if ndim <= 4 {
            std::iter::chain(a_new_named_point_set_orbits, b_new_named_point_set_orbits).collect()
        } else {
            vec![]
        };

        Ok(Self {
            id: crate::product_id(factors.iter().map(|f| &f.id)),

            coxeter_mirrors,
            group,

            named_point_action,
            named_point_vectors,
            named_point_unit_vectors,

            axis_action,
            axis_vectors,

            factors,

            named_point_set_orbits,
        })
    }

    /// Returns the number of axes on the puzzle.
    pub fn len(&self) -> usize {
        self.axis_vectors.len()
    }
    /// Returns the number of named points on the puzzle.
    pub fn named_points_count(&self) -> usize {
        self.named_point_vectors.len()
    }

    pub fn build(
        &self,
        build_ctx: &BuildCtx,
        warn_fn: &mut impl FnMut(eyre::Report),
    ) -> Result<Arc<TwistSystem>> {
        let names = ProductNamedPointBasedNames::product(
            self.factors.iter().map(|f| f.names.clone()).collect(),
        );

        let mut components = ComponentList::new();
        components.insert(Arc::new(NdEuclidAxisVectors::from_vectors(
            self.ndim(),
            self.axis_vectors.clone(),
        )));

        let axes = Arc::new(AxisSystem {
            names: Arc::new(names.build_member_names()?),
            orbits: vec![], // technically exists, but not necessary
            components,
        });

        let named_point_names = Arc::new(names.build_named_point_names()?);

        let mut components = ComponentList::new();
        components.insert(Arc::new(SymmetricTwistSystemComponent {
            axes: Arc::clone(&axes),
            group: self.group.clone(),
            coxeter_mirrors: self.coxeter_mirrors.clone(),
            axis_action: self.axis_action.clone(),

            axis_undeorbiters: Arc::new(self.build_axis_undeorbiters()?),
            axis_orbits: Arc::new(self.build_axis_orbits(
                &axes.names,
                &named_point_names,
                warn_fn,
            )?),

            named_point_action: self.named_point_action.clone(),
            named_point_names,
            named_point_vectors: Arc::new(self.named_point_vectors.clone()),
        }));

        // TODO: verify that named points are sufficient to describe
        //       symmetry group elements

        Ok(Arc::new(TwistSystem {
            id: self.id.clone(),
            name: self.name(),
            components,
            axes: Arc::clone(&axes),
            axis_from_family: Box::new(move |family_str| {
                // TODO: correct number of underscores
                let axis_name = match family_str.split_once('_') {
                    Some((first, _)) => first,
                    None => family_str,
                };
                axes.names.lookup(axis_name)
            }),
            ..TwistSystem::new_empty()
        }))
    }

    /// Returns an iterator over all axis orbits, each paired with the ID of the
    /// first axis in that orbit.
    pub fn axis_orbits(&self) -> impl Iterator<Item = &AxisOrbit> {
        self.factors.iter().flat_map(|factor| &factor.axis_orbits)
    }

    pub fn axis_from_name(&self, axis_name: &str) -> Option<Axis> {
        let (SequentialLowercaseName(factor_index), rest) = if self.factors.len() == 1 {
            (SequentialLowercaseName(0), axis_name)
        } else {
            hypuz_notation::family::strip_sequential_lowercase_prefix(axis_name)?
        };

        self.factors
            .get(factor_index as usize)?
            .names
            .member_from_name(rest)
            .ok()
    }

    pub fn orbit_containing_axis(&self, axis: Axis) -> Option<usize> {
        // This could be faster, but it doesn't need to be
        self.axis_orbits()
            .position(|orbit| orbit.axes().contains(&axis))
    }

    fn build_axis_undeorbiters(&self) -> Result<PerAxis<(GroupElementId, usize)>> {
        let mut ret = PerAxis::new_with_len(self.len());

        for (orbit_index, orbit) in self.axis_orbits().enumerate() {
            ret[orbit.first()] = (GroupElementId::IDENTITY, orbit_index);
            hypergroup::orbit(
                (orbit.first(), GroupElementId::IDENTITY),
                self.group.generators(),
                |&(ax, undeorbiter), &g| {
                    let new_ax = self.axis_action.act(g, ax);
                    (new_ax != orbit.first() && ret[new_ax].0 == GroupElementId::IDENTITY).then(
                        || {
                            let new_undeorbiter = self.group.compose(g, undeorbiter);
                            ret[new_ax] = (new_undeorbiter, orbit_index);
                            (new_ax, new_undeorbiter)
                        },
                    )
                },
            );

            // Sanity check that we didn't miss any
            #[cfg(debug_assertions)]
            let axes_in_orbit =
                Axis::iter_range(orbit.first().to_index()..orbit.first().to_index() + orbit.len);
            for ax in axes_in_orbit.skip(1) {
                assert_ne!(ret[ax].0, GroupElementId::IDENTITY);
                assert_eq!(ret[ax].1, orbit_index);
            }
        }

        Ok(ret)
    }

    fn build_axis_orbits(
        &self,
        axis_names: &Names<Axis>,
        named_point_names: &Names<NamedPoint>,
        warn_fn: &mut impl FnMut(eyre::Report),
    ) -> Result<Vec<SymmetricTwistSystemAxisOrbit>> {
        let mut ret = vec![];
        for orbit in self.axis_orbits() {
            let first_axis_vector = &self.axis_vectors[orbit.first()];

            let mut subgroup_solver = SubgroupConstraintSolver::new(
                SubgroupAction::from_subgroup_predicate(&self.named_point_action, |e| {
                    self.axis_action.act(e, orbit.first()) == orbit.first()
                })?,
            );

            let stabilizer_twist_families = match self.group.ndim() {
                // gizmo pole distance doesn't matter
                3 => &[(NamedPointSet::EMPTY, 0.0)],
                // for 3D
                4 => &*orbit.stabilizer_twists,
                _ => &[],
            };

            let stabilizer_twists = stabilizer_twist_families
                .iter()
                .map(|(secondary, distance)| {
                    let get_twist_name = || {
                        StabilizerFamily {
                            primary: orbit.first(),
                            secondary: secondary.clone(),
                        }
                        .name(axis_names, named_point_names)
                    };

                    if secondary.len() > 3 {
                        bail!(
                            "cannot compute stabilizer unit twist transform \
                             for more than 3 axes; this is a program limitation",
                        );
                    }

                    let coset = subgroup_solver
                        .solve(&hypergroup::ConstraintSet::from_iter(
                            secondary
                                .iter()
                                .circular_tuple_windows()
                                .map(|(from, to)| hypergroup::Constraint { from, to }),
                        ))
                        .ok_or_else(|| {
                            eyre!(
                                "stabilizer twist {:?} imposes unsatisfiable constraints",
                                get_twist_name(),
                            )
                        })?;

                    let unit_twist_transform = if secondary.is_empty() {
                        unit_twist_transform(&self.group, &coset, &[first_axis_vector])
                    } else {
                        let secondary_vector = secondary.vector(&self.named_point_vectors);
                        let stabilized_vectors = &[first_axis_vector, &secondary_vector];
                        unit_twist_transform(&self.group, &coset, stabilized_vectors)
                    }
                    .wrap_err_with(|| {
                        format!(
                            "error calculating unit twist transform \
                             for stabilizer twist {:?}",
                            get_twist_name(),
                        )
                    })?;

                    Ok((secondary.clone(), unit_twist_transform, *distance))
                })
                .filter_map(|result| match result {
                    Ok(ok) => Some(ok),
                    Err(e) => {
                        warn_fn(e);
                        None
                    }
                })
                .collect();

            ret.push(SymmetricTwistSystemAxisOrbit {
                first: orbit.first(),
                len: orbit.len,
                subgroup_solver: Mutex::new(subgroup_solver),
                stabilizer_twists,
            });
        }

        Ok(ret)
    }

    pub fn name(&self) -> String {
        crate::product_name(self.factors.iter().map(|f| &f.name))
    }
}

/// Returns the unique minimal clockwise generator for a coset, or `None` if
/// there is not one.
///
/// `stabilized_vectors` must be a list of vectors of length `ndim-2`, and
/// is used to define "clockwise."
fn unit_twist_transform(
    group: &IsometryGroup,
    stabilizer: &ConjugateCoset,
    stabilized_vectors: &[&Vector],
) -> Result<UniqueMinimalClockwiseGenerator> {
    if stabilized_vectors.len() + 2 != group.ndim() as usize {
        bail!("`stabilized_vectors` must have length ndim-2");
    }
    let nontrivial_rotations = stabilizer
        .elements()
        .into_iter()
        .filter(|&e| e != GroupElementId::IDENTITY)
        .filter(|&e| !group.is_reflection(e))
        .collect_vec();
    let order =
        NonZeroI32::new(nontrivial_rotations.len() as i32 + 1).ok_or_eyre("math is broken")?;
    // TODO: actually check that min_rotation generates the whole group
    let (mut min_group_element, min_rotation) = nontrivial_rotations
        .into_iter()
        .filter_map(|e| Some((e, group.motor(e).normalize()?)))
        .max_by_float_key(|(_e, m)| m.scalar().abs())
        .ok_or_eyre("empty coset")?;
    let arbitrary_nonparallel_vector = Vector::unit(
        (0..group.ndim())
            .min_by_float_key(|&i| {
                stabilized_vectors
                    .iter()
                    .map(|v| v.get(i).abs())
                    .max_float()
                    .unwrap_or(0.0)
            })
            .unwrap_or(0),
    );
    let orientation = Matrix::from_cols(
        std::iter::chain(
            stabilized_vectors.iter().copied(),
            [
                &arbitrary_nonparallel_vector,
                &min_rotation.transform(&arbitrary_nonparallel_vector),
            ],
        )
        .collect_vec(), // Chain does not impl ExactSizeIterator
    )
    .determinant();
    if orientation > 0.0 {
        min_group_element = group.inverse(min_group_element);
    }
    Ok(UniqueMinimalClockwiseGenerator {
        element: min_group_element,
        order,
    })
}

use std::{
    collections::{HashMap, HashSet, VecDeque},
    num::NonZeroI32,
    sync::Arc,
};

use eyre::{Context, OptionExt, Result, bail, eyre};
use hypergroup::{
    ConjugateCoset, Constraint, ConstraintSet, ConstraintSolver, CoxeterMatrix, GroupAction,
    GroupElementId, GroupError, IsometryGroup, PerGenerator, PerGroupElement, SubgroupAction,
    SubgroupConstraintSolver,
};
use hypermath::{APPROX, Float, Matrix, Point, Sign, Vector, VectorRef, num::Euclid};
use hyperpuzzle_core::{
    Axis, AxisSystem, IndexOverflow, NameSpecBiMap, NameSpecBiMapBuilder, Orbit, PerAxis, TiMask,
    TypedIndex, TypedIndexIter,
};
use hypuz_notation::{AxisLayersInfo, Str};
use hypuz_util::{FloatMinMaxByIteratorExt, FloatMinMaxIteratorExt};
use itertools::Itertools;
use parking_lot::Mutex;
use smallvec::{SmallVec, smallvec};

use crate::{
    AxisOrbitSpec, NamedPoint, NamedPointOrbitSpec, NamedPointSet, PerNamedPoint, StabilizerFamily,
    SymmetricTwistSystemAxisOrbit, UniqueMinimalClockwiseGenerator, names::NameBiMap,
};

/// Axis system of a puzzle under construction.
#[derive(Debug)]
pub(super) struct ProductPuzzleAxes {
    /// Grip group.
    pub group: IsometryGroup,
    /// Coxeter matrix for the grip group.
    pub coxeter_matrix: CoxeterMatrix,

    /// Action of the grip group on named points.
    pub named_point_action: GroupAction<NamedPoint>,
    /// Vector for each named point.
    pub named_point_vectors: Arc<PerNamedPoint<Vector>>,
    /// Named point orbits.
    pub named_point_orbits: Vec<NamedPointOrbit>,

    /// Action of the grip group on axes.
    pub axis_action: GroupAction<Axis>,
    /// Vector for each axis.
    ///
    /// The vector is not necessarily normalized. Its magnitude determines the
    /// placement of twist gizmos in 3D and 4D. For a facet-turning puzzle, each
    /// axis vector will typically be scaled to match the distance of its
    /// corresponding facet.
    pub axis_vectors: Arc<PerAxis<Vector>>,
    /// Axis orbits.
    pub axis_orbits: Vec<AxisOrbit>,

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

impl ProductPuzzleAxes {
    /// Returns the number of the dimensions of the puzzle.
    pub fn ndim(&self) -> u8 {
        self.group.ndim()
    }

    /// Constructs the empty axis system, which is the identity of the direct
    /// product.
    pub fn direct_product_identity() -> Self {
        Self {
            group: IsometryGroup::trivial(),
            coxeter_matrix: CoxeterMatrix::trivial(),

            named_point_action: GroupAction::trivial(),
            named_point_vectors: Arc::new(PerNamedPoint::new()),
            named_point_orbits: vec![],

            axis_action: GroupAction::trivial(),
            axis_vectors: Arc::new(PerAxis::new()),
            axis_orbits: vec![],

            named_point_set_orbits: vec![],
        }
    }

    pub fn new(
        coxeter_matrix: CoxeterMatrix,
        group: IsometryGroup,
        axis_orbits: &[AxisOrbitSpec],
        named_point_orbits: &[NamedPointOrbitSpec],
        named_point_set_orbits: &[(Vec<Str>, Float)],
        warn_fn: &mut impl FnMut(eyre::Report),
    ) -> Result<Self> {
        let original_generators = group.generator_motors();

        // Shuffling group generators improves average word length, making some
        // group operations faster.
        let group = crate::shuffle_group_generators(&group, &mut rand::rng())?;

        let mut all_named_point_names = NameBiMap::<NamedPoint>::new();
        let mut named_point_vectors = PerNamedPoint::new();
        let mut new_named_point_orbits = vec![];
        let mut id_offset = 0;
        for orbit in named_point_orbits {
            let mut names = vec![];
            for (vector, name) in
                orbit.named_point_vectors(original_generators, |e| warn_fn(eyre!(e)))
            {
                named_point_vectors.push(vector.clone())?;
                names.push(name.clone());
                all_named_point_names.push(name.clone())?;
            }
            new_named_point_orbits.push(NamedPointOrbit {
                len: orbit.len(),
                prefix: hypuz_notation::family::SequentialLowercaseName(0),
                id_offset,
                names: Arc::new(names),
            });
            id_offset += orbit.len();
        }

        let named_point_points = named_point_vectors.map_ref(|_, v| Point(v.clone()));
        let named_point_action = group.action_on_points(&named_point_points)?;

        let named_point_from_name = |name: &Str| {
            all_named_point_names
                .name_to_id(name)
                .ok_or_else(|| eyre!("no named point with name {name:?}"))
        };

        let mut all_axis_names = NameBiMap::<Axis>::new();
        let mut axis_vectors = PerAxis::new();
        let mut new_axis_orbits = vec![];
        let mut id_offset = 0;
        for orbit in axis_orbits {
            let mut names = vec![];
            for (vector, name) in
                orbit.named_axis_vectors(original_generators, |e| warn_fn(eyre!(e)))
            {
                axis_vectors.push(vector.clone())?;
                names.push(name.clone());
                all_axis_names.push(name.clone())?;
            }
            new_axis_orbits.push(AxisOrbit {
                len: orbit.len(),
                prefix: hypuz_notation::family::SequentialLowercaseName(0),
                id_offset,
                max_layer: orbit.layer_count().try_into()?,
                generator_sequences: Arc::new(
                    orbit
                        .names
                        .iter()
                        .map(|(abbr_gen_seq, _)| abbr_gen_seq.clone())
                        .collect(),
                ),
                names: Arc::new(names),
                stabilizer_action: SubgroupAction::trivial(), // easier to add once we have `axis_action`
                stabilizer_twists: orbit
                    .stabilizer_sets
                    .iter()
                    .map(|(named_points, distance)| {
                        let named_point_set = NamedPointSet::new(
                            named_points
                                .iter()
                                .map(named_point_from_name)
                                .try_collect()?,
                        )?;
                        eyre::Ok((named_point_set, *distance))
                    })
                    .try_collect()?,
            });
            id_offset += orbit.len();
        }

        let axis_points = axis_vectors.map_ref(|_, v| Point(v.clone()));
        let axis_action = group.action_on_points(&axis_points)?;

        for orbit in &mut new_axis_orbits {
            let first_axis = orbit.first();
            orbit.stabilizer_action =
                SubgroupAction::from_subgroup_predicate(&named_point_action, |e| {
                    axis_action.act(e, first_axis) == first_axis
                })?;
        }

        let axis_from_name = |name: &Str| {
            all_axis_names
                .name_to_id(name)
                .ok_or_else(|| eyre!("no axis with name {name:?}"))
        };

        let named_point_set_orbits: Vec<(NamedPointSet, Float)> = std::iter::chain(
            new_named_point_orbits.iter().map(|orbit| {
                Ok((
                    NamedPointSet::new(smallvec![orbit.first()])?,
                    named_point_vectors[orbit.first()].mag(),
                ))
            }),
            named_point_set_orbits.iter().map(|(names, distance)| {
                eyre::Ok((
                    NamedPointSet::new(names.iter().map(named_point_from_name).try_collect()?)?,
                    *distance,
                ))
            }),
        )
        .try_collect()?;

        Ok(Self {
            group,
            coxeter_matrix,

            named_point_action,
            named_point_vectors: Arc::new(named_point_vectors),
            named_point_orbits: new_named_point_orbits,

            axis_action,
            axis_vectors: Arc::new(axis_vectors),
            axis_orbits: new_axis_orbits,

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

        let named_point_vectors = std::iter::chain(
            a.named_point_vectors
                .iter_values()
                .map(|v| crate::lift_vector_by_ndim(v, 0, a.ndim(), b.ndim())),
            b.named_point_vectors
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

        let lift_b_axis = |ax: Axis| Axis(ax.0 + a.len() as u16);
        let lift_b_point_set =
            |points: &NamedPointSet| points.offset_ids_by(a.named_points_count());

        let a_new_named_point_set_orbits = a.named_point_set_orbits.iter().cloned();
        let b_new_named_point_set_orbits = b
            .named_point_set_orbits
            .iter()
            .map(|(set, distance)| (lift_b_point_set(set), *distance))
            .collect_vec();

        let named_point_orbits = std::iter::chain(
            a.named_point_orbits.iter().cloned().map(eyre::Ok),
            b.named_point_orbits.iter().cloned().map(|b_orbit| {
                Ok(b_orbit
                    .offset_ids_by(a.len())?
                    .offset_prefix_by(a.prefix_count()))
            }),
        )
        .try_collect()?;
        let axis_orbits = std::iter::chain(
            a.axis_orbits.iter().map(|a_orbit| {
                a_orbit
                    .clone()
                    .right_multiply_by(b, ndim, &b_new_named_point_set_orbits)
            }),
            b.axis_orbits
                .iter()
                .map(|b_orbit| b_orbit.clone().left_multiply_by(a, ndim)),
        )
        .try_collect()?;

        let named_point_set_orbits = if ndim <= 4 {
            std::iter::chain(a_new_named_point_set_orbits, b_new_named_point_set_orbits).collect()
        } else {
            vec![]
        };

        Ok(Self {
            coxeter_matrix: CoxeterMatrix::direct_product(&a.coxeter_matrix, &b.coxeter_matrix)?,
            group,

            named_point_action,
            named_point_vectors: Arc::new(named_point_vectors),
            named_point_orbits,

            axis_action,
            axis_vectors: Arc::new(axis_vectors),
            axis_orbits,

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

    /// Returns the number of lowercase prefixes that are in use for axis sets.
    pub fn prefix_count(&self) -> u32 {
        self.axis_orbits
            .iter()
            .map(|orbit| orbit.prefix.0 + 1)
            .max()
            .unwrap_or(0)
    }

    pub fn build_axis_system(&self) -> Result<AxisSystem> {
        let mut names = NameSpecBiMapBuilder::new();
        for orbit in &self.axis_orbits {
            for (id, name) in std::iter::zip(orbit.axes(), &*orbit.names) {
                names.set(id, Some(format!("{}{}", orbit.prefix, name)))?;
            }
        }
        let names = Arc::new(names.build(self.len()).ok_or_eyre("missing axis name")?);

        let orbits = self
            .axis_orbits
            .iter()
            .map(|orbit| Orbit {
                elements: Arc::new(orbit.axes().map(Some).collect()),
                generator_sequences: Arc::clone(&orbit.generator_sequences),
            })
            .collect();

        Ok(AxisSystem { names, orbits })
    }

    pub fn build_axis_undeorbiters(&self) -> PerAxis<(GroupElementId, usize)> {
        let mut ret = PerAxis::new_with_len(self.len());

        for (orbit_index, orbit) in self.axis_orbits.iter().enumerate() {
            ret[orbit.first()] = (GroupElementId::IDENTITY, orbit_index);
            hypergroup::orbit(
                (orbit.first(), GroupElementId::IDENTITY),
                self.group.generators(),
                |&(ax, undeorbiter), &g| {
                    let new_ax = self.axis_action.act(g, ax);
                    if new_ax != orbit.first() && ret[new_ax].0 == GroupElementId::IDENTITY {
                        let new_undeorbiter = self.group.compose(g, undeorbiter);
                        ret[new_ax] = (new_undeorbiter, orbit_index);
                        Some((new_ax, new_undeorbiter))
                    } else {
                        None
                    }
                },
            );

            // Sanity check that we didn't miss any
            #[cfg(debug_assertions)]
            for ax in orbit.axes().skip(1) {
                assert_ne!(ret[ax].0, GroupElementId::IDENTITY);
                assert_eq!(ret[ax].1, orbit_index);
            }
        }

        ret
    }

    pub fn build_named_point_names(&self) -> Result<NameSpecBiMap<NamedPoint>> {
        let mut names = NameSpecBiMapBuilder::new();
        for orbit in &self.named_point_orbits {
            for (id, name) in std::iter::zip(orbit.named_points(), &*orbit.names) {
                names.set(id, Some(format!("{}{}", orbit.prefix, name)))?;
            }
        }
        names
            .build(self.len())
            .ok_or_eyre("missing named point name")
    }

    pub fn build_axis_layers(&self) -> PerAxis<AxisLayersInfo> {
        self.axis_orbits
            .iter()
            .flat_map(|orbit| {
                orbit.axes().map(|_| AxisLayersInfo {
                    max_layer: orbit.max_layer,
                    allow_negatives: false, // TODO: allow negatives on some axes
                })
            })
            .collect()
    }

    pub fn build_axis_orbits(
        &self,
        axis_names: &NameSpecBiMap<Axis>,
        named_point_names: &NameSpecBiMap<NamedPoint>,
        warn_fn: &mut impl FnMut(eyre::Report),
    ) -> Result<Vec<SymmetricTwistSystemAxisOrbit>> {
        self.axis_orbits
            .iter()
            .map(|orbit| {
                let first_axis_vector = &self.axis_vectors[orbit.first()];

                let mut subgroup_solver = SubgroupConstraintSolver::new(
                    SubgroupAction::from_subgroup_predicate(&self.named_point_action, |e| {
                        self.axis_action.act(e, orbit.first()) == orbit.first()
                    })?,
                );

                let stabilizer_twist_families = match self.group.ndim() {
                    3 => &[(NamedPointSet::EMPTY, 0.0)], // gizmo pole distance doesn't matter for 3D
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
                                    "no unique minimal clockwise generator \
                                     for stabilizer twist {:?}",
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

                Ok(SymmetricTwistSystemAxisOrbit {
                    first: orbit.first(),
                    subgroup_solver: Mutex::new(subgroup_solver),
                    stabilizer_twists,
                })
            })
            .try_collect()
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
                    .map(|v| v[i].abs())
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

/// Orbit of named points.
///
/// This type is mostly reference-counted and thus relatively cheap to clone.
#[derive(Debug, Clone)]
pub struct NamedPointOrbit {
    /// Number of named points in the orbit.
    pub len: usize,
    /// Sequential lowercase prefix for the orbit.
    ///
    /// This may be shared among other orbits.
    pub prefix: hypuz_notation::family::SequentialLowercaseName,
    /// ID offset of the named points in the orbit.
    ///
    /// IDs within an orbit always count starting from 0, but the puzzle may
    /// have multiple sets and so puzzle-facing IDs for named points in this set
    /// must start counting from this offset.
    pub id_offset: usize,
    /// Name for each named point in the orbit, not including its prefix.
    pub names: Arc<Vec<String>>,
}

impl NamedPointOrbit {
    /// Offsets all named point IDs by an additional amount.
    pub fn offset_ids_by(mut self, additional_id_offset: usize) -> Result<Self, IndexOverflow> {
        self.id_offset += additional_id_offset;
        Axis::try_iter_range(self.id_offset..self.id_offset + self.len)?; // check for overflow
        Ok(self)
    }

    /// Offsets the named point prefix by an additional amount.
    pub fn offset_prefix_by(mut self, additional_prefix_offset: u32) -> Self {
        self.prefix.0 += additional_prefix_offset;
        self
    }

    /// Returns the first named point in the orbit.
    pub fn first(&self) -> NamedPoint {
        NamedPoint::try_from_index(self.id_offset)
            .expect("overflow should have been caught on construction")
    }

    /// Returns an iterator over the named points in the orbit.
    pub fn named_points(&self) -> TypedIndexIter<NamedPoint> {
        NamedPoint::iter_range(self.id_offset..self.id_offset + self.len)
    }
}

/// Orbit of axes.
///
/// This type is mostly reference-counted and thus relatively cheap to clone.
#[derive(Debug, Clone)]
pub struct AxisOrbit {
    /// Number of axes in the orbit.
    pub len: usize,
    /// Sequential lowercase prefix for the orbit.
    ///
    /// This may be shared among other orbits.
    pub prefix: hypuz_notation::family::SequentialLowercaseName,
    /// ID offset of the axes in the orbit.
    ///
    /// IDs within an orbit always count starting from 0, but the puzzle may
    /// have multiple sets and so puzzle-facing IDs for axes in this set must
    /// start counting from this offset.
    pub id_offset: usize,
    /// Number of layers on each axis, which is equivalent to the maximum layer
    /// number.
    pub max_layer: u16,
    /// Generator sequence for each axis in the orbit.
    pub generator_sequences: Arc<Vec<hypergroup::AbbrGenSeq>>,
    /// Name for each axis in the orbit, not including its prefix.
    pub names: Arc<Vec<String>>,
    /// Action on named points of the stabilizer subgroup with respect to the
    /// first axis in the orbit.
    pub stabilizer_action: SubgroupAction<NamedPoint>,
    /// Nontrivial stabilizer twist families, along with their gizmo pole
    /// distances.
    ///
    /// Here, "nontrivial" means that the named point set is nonempty. The named
    /// point set is typically empty for 3D twists, but these do not need to be
    /// tracked.
    ///
    /// These are the stabilizer twist families in 4D puzzles. Because they are
    /// not needed in higher dimensions, this list is made empty in 5D+.
    pub stabilizer_twists: Vec<(NamedPointSet, Float)>,
}

impl AxisOrbit {
    fn left_multiply_by(mut self, lhs: &ProductPuzzleAxes, total_ndim: u8) -> Result<Self> {
        self.prefix.0 += lhs.prefix_count();

        self.id_offset += lhs.len();
        Axis::try_iter_range(self.id_offset..self.id_offset + self.len)?; // check for overflow

        self.stabilizer_action =
            SubgroupAction::direct_product_left(&lhs.named_point_action, self.stabilizer_action)?;

        for (named_point_set, _gizmo_pole_distance) in &mut self.stabilizer_twists {
            named_point_set.offset_ids_by(lhs.named_points_count());
        }

        // Add new stabilizer twists from `lhs` named point sets.
        self.update_stabilizer_twists(total_ndim, &lhs.named_point_set_orbits);

        Ok(self)
    }

    fn right_multiply_by(
        mut self,
        rhs: &ProductPuzzleAxes,
        total_ndim: u8,
        new_named_point_set_orbits: &[(NamedPointSet, Float)], // must use named point IDs for product
    ) -> Result<Self> {
        self.stabilizer_action =
            SubgroupAction::direct_product_right(self.stabilizer_action, &rhs.named_point_action)?;

        // Add new stabilizer twists from `rhs` named point sets.
        self.update_stabilizer_twists(total_ndim, new_named_point_set_orbits);

        Ok(self)
    }

    fn update_stabilizer_twists(
        &mut self,
        total_ndim: u8,
        new_named_set_orbits: &[(NamedPointSet, Float)],
    ) {
        if total_ndim <= 4 {
            self.stabilizer_twists
                .extend_from_slice(new_named_set_orbits);
        } else {
            self.stabilizer_twists.clear();
        }
    }
}

impl AxisOrbit {
    /// Returns the first axis in the orbit.
    pub fn first(&self) -> Axis {
        Axis::try_from_index(self.id_offset)
            .expect("overflow should have been caught on construction")
    }

    /// Returns an iterator over the axes in the orbit.
    pub fn axes(&self) -> TypedIndexIter<Axis> {
        Axis::iter_range(self.id_offset..self.id_offset + self.len)
    }
}

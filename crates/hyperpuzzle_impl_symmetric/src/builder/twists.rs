use std::num::NonZeroI32;
use std::sync::Arc;

use eyre::{Context, OptionExt, Result, bail, ensure, eyre};
use hypergroup::{
    AbbrGenSeq, ConjugateCoset, GroupAction, GroupElementId, IsometryGroup, SubgroupAction,
    SubgroupConstraintSolver,
};
use hypermath::{APPROX, Float, Matrix, Point, Subspace, Vector, VectorRef};
use hyperpuzzle_core::catalog::BuildCtx;
use hyperpuzzle_core::{
    Axis, AxisSystem, CatalogId, CatalogObject, ComponentList, IndexOverflow, Names, PerAxis,
    TwistSystem, TypedIndex, TypedIndexIter,
};
use hyperpuzzle_impl_nd_euclid::NdEuclidAxisVectors;
use hypuz_notation::charsets::CharSet;
use hypuz_notation::family::SequentialLowercaseName;
use hypuz_util::{FloatMinMaxByIteratorExt, FloatMinMaxIteratorExt};
use itertools::Itertools;
use parking_lot::Mutex;
use smallvec::smallvec;

use crate::names::{FactorTwistSystemNames, PrefixFreeBiMap, ProductTwistSystemNames};
use crate::{
    FactorTwistSystemSpec, NamedPoint, NamedPointSet, PerNamedPoint, StabilizerFamily,
    SymmetricTwistSystemAxisOrbit, SymmetricTwistSystemComponent, UniqueMinimalClockwiseGenerator,
};

#[derive(Debug, Clone)]
struct TwistSystemFactor {
    id: CatalogId,
    names: FactorTwistSystemNames,
    axis_orbits: Vec<AxisOrbit>,
    named_point_orbits: Vec<NamedPointOrbit>,
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

#[derive(Debug, Clone)]
struct NamedPointOrbit {
    len: usize,
    id_offset: usize,
    abbr_gen_seqs: Vec<AbbrGenSeq>,
}

impl NamedPointOrbit {
    fn first(&self) -> Result<NamedPoint, IndexOverflow> {
        NamedPoint::try_from_index(self.id_offset)
    }

    fn offset_ids_by(&self, named_point_id_offset: usize) -> Result<Self, IndexOverflow> {
        let new_id_offset = self.id_offset + named_point_id_offset;
        Axis::try_iter_range(new_id_offset..new_id_offset + self.len)?; // check for overflow
        Ok(Self {
            len: self.len,
            id_offset: new_id_offset,
            abbr_gen_seqs: self.abbr_gen_seqs.clone(),
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
            id: crate::product_id(&[]),
            group: IsometryGroup::trivial(),
            coxeter_mirrors: vec![],
            named_point_action: GroupAction::trivial(),
            named_point_vectors: PerNamedPoint::new(),
            axis_action: GroupAction::trivial(),
            axis_vectors: PerAxis::new(),
            factors: vec![],
            named_point_set_orbits: vec![],
        }
    }

    /// Constructs a product twist system builder with a single factor.
    pub fn new_factor(
        id: CatalogId,
        spec: &FactorTwistSystemSpec,
        warn_fn: &mut impl FnMut(eyre::Report),
    ) -> Result<Self> {
        let factor_id = id;

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
            // Shuffle group generators to improve average word length, making some
            // group operations faster.
            group = crate::shuffle_group_generators(&unshuffled_group, &mut rand::rng())
                .wrap_err("error shuffling twist symmetry generators")?;
        } else {
            coxeter_mirrors = vec![];
            group = IsometryGroup::trivial_with_ndim(spec.ndim);
        }

        let mut named_point_vectors = PerNamedPoint::new();
        let mut normalized_named_point_vectors = PerNamedPoint::new();
        let mut named_point_names = PrefixFreeBiMap::new();
        let mut named_point_orbits = vec![];
        let mut named_point_id_offset = 0;
        for orbit in spec
            .named_point_orbits
            .iter()
            .sorted_by_cached_key(|orbit| orbit.min_name())
        {
            let mut abbr_gen_seqs = vec![];
            let sorted_points_in_orbit = orbit
                .orbit_members
                .iter()
                .sorted_by_key(|point| &point.name);
            for point in sorted_points_in_orbit {
                // Validate name
                let name = point.name.clone();
                if let Some(bad_char) = name.chars().find(|c| {
                    !matches!(
                        hypuz_notation::charsets::classify(*c),
                        Some(CharSet::UppercaseLatin | CharSet::UppercaseGreek),
                    )
                }) {
                    bail!("named point {name:?} contains disallowed char {bad_char:?}");
                }

                named_point_vectors.push(point.vector.clone())?;
                normalized_named_point_vectors.push(
                    point
                        .vector
                        .normalize()
                        .ok_or_eyre("named point cannot be zero")?,
                )?;
                named_point_names.push(name)?;
                abbr_gen_seqs.push(point.abbr_gen_seq.clone());
            }
            named_point_orbits.push(NamedPointOrbit {
                len: orbit.len(),
                id_offset: named_point_id_offset,
                abbr_gen_seqs,
            });
            named_point_id_offset += orbit.len();
        }

        let points = named_point_vectors.map_ref(|_, v| Point(v.clone()));
        let named_point_action = group.action_on_points(&points)?;

        let mut names = FactorTwistSystemNames::new(Arc::new(named_point_names))?;

        let mut axis_orbits = vec![];
        let mut axis_undeorbiters = PerAxis::new();
        let mut axis_which_orbit = PerAxis::new();
        let mut axis_vectors = PerAxis::new();
        let mut axis_id_offset = 0;
        for (orbit_index, orbit) in spec.axis_orbits.iter().enumerate() {
            let new_axis_names = expand_and_name_axis_orbit(
                &group,
                &normalized_named_point_vectors,
                &named_point_action,
                &orbit.vector,
            )?;
            axis_orbits.push(AxisOrbit {
                len: new_axis_names.len(),
                id_offset: axis_id_offset,
                stabilizer_twists: vec![], // will be populated later
            });
            axis_id_offset += new_axis_names.len();
            for (undeorbiter, axis_vector, axis_name) in new_axis_names {
                axis_undeorbiters.push(undeorbiter)?;
                axis_which_orbit.push(orbit_index)?;
                axis_vectors.push(axis_vector)?;
                names.add_axis(&orbit.prefix, axis_name)?;
            }
        }

        let axis_points = axis_vectors.map_ref(|_, v| Point(v.clone()));
        let axis_action = group.action_on_points(&axis_points)?;

        // Populate stabilizer twists
        for orbit in &spec.stabilizer_twist_orbits {
            let axis = names.axis_from_name(&orbit.axis_name)?;
            let stabilized_points = NamedPointSet::new(
                orbit
                    .named_points
                    .iter()
                    .map(|s| names.named_point_from_name(s))
                    .try_collect()?,
            )?;
            let axis_orbit_index = axis_which_orbit[axis];
            axis_orbits[axis_orbit_index]
                .stabilizer_twists
                .push((stabilized_points, orbit.gizmo_pole_distance));
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

        let id = crate::product_id(std::slice::from_ref(&factor_id));
        let factor = TwistSystemFactor {
            id: factor_id,
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
            id: crate::product_id(&factors.iter().map(|f| f.id.clone()).collect_vec()),

            coxeter_mirrors,
            group,

            named_point_action,
            named_point_vectors,

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
        let names = ProductTwistSystemNames::product(
            self.factors.iter().map(|f| f.names.clone()).collect(),
        );

        let mut components = ComponentList::new();
        components.insert(Arc::new(NdEuclidAxisVectors::from_vectors(
            self.ndim(),
            self.axis_vectors.clone(),
        )));

        let axes = Arc::new(AxisSystem {
            names: Arc::new(names.build_axis_names()?),
            orbits: vec![], // technically exists, but not necessary
            components,
        });

        let named_point_names = Arc::new(names.build_named_point_names()?);

        let mut components = ComponentList::new();
        components.insert(Arc::new(SymmetricTwistSystemComponent {
            axes: Arc::clone(&axes),
            group: self.group.clone(),
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
            .axis_from_name(rest)
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

            ret.push(SymmetricTwistSystemAxisOrbit {
                first: orbit.first(),
                subgroup_solver: Mutex::new(subgroup_solver),
                stabilizer_twists,
            });
        }

        Ok(ret)
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

/// Generates an axis orbit with a canonical name for each axis based on its
/// nearest named points.
///
/// The named point locations **must** be normalized.
fn expand_and_name_axis_orbit(
    group: &IsometryGroup,
    normalized_named_point_locations: &PerNamedPoint<Vector>,
    named_point_action: &GroupAction<NamedPoint>,
    axis_vector: &Vector,
) -> Result<Vec<(GroupElementId, Vector, Vec<Vec<NamedPoint>>)>> {
    let orbit = group.orbit_geometric(axis_vector.clone());

    let Some(normalized_axis_vector) = axis_vector.normalize() else {
        // No named points needed! Hopefully the axis orbit has a prefix;
        // otherwise the axis name will be empty.
        return Ok(vec![(
            GroupElementId::IDENTITY,
            axis_vector.clone(),
            vec![],
        )]);
    };
    let first_axis_vector = normalized_axis_vector.clone();

    if orbit.len() == 1 {
        // No named points needed! Same as above
        return Ok(vec![(
            GroupElementId::IDENTITY,
            axis_vector.clone(),
            vec![],
        )]);
    }

    // Name each axis based on the nearest named points.
    let mut axis_names: PerAxis<Vec<Vec<NamedPoint>>> = orbit.iter().map(|_| vec![]).collect();
    let distances: PerNamedPoint<Float> =
        normalized_named_point_locations.map_ref(|_, loc| (&first_axis_vector - loc).mag2());

    let mut last_multiplicity = axis_names.len(); // all names are the same at the start
    let mut candidates: Vec<NamedPoint> =
        normalized_named_point_locations.iter_keys().collect_vec();
    let mut subspace = Subspace::new();
    loop {
        // Select all the named points that are equally closest to the axis.
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

        // Append that set of named points to the name for each axis.
        for (&(elem, _), axis_name) in orbit.iter().zip(axis_names.iter_values_mut()) {
            axis_name.push(
                closest_points
                    .iter()
                    .map(|&p| named_point_action.act(elem, p))
                    .sorted()
                    .collect(),
            );
        }

        // Check the "multiplicity" of each name; i.e., how many axes have the
        // same name. This should be the same for all names.
        let first_axis_name = &axis_names[Axis(0)];
        let multiplicity = axis_names
            .iter_values()
            .filter(|&name| name == first_axis_name)
            .count();
        // Check that all names have the same multiplicity, so we only need to
        // check the first name.
        debug_assert!(
            axis_names
                .iter_values()
                .counts()
                .into_iter()
                .all(|(_, count)| count == multiplicity)
        );
        // If the names are all unique, then we're done!
        if multiplicity == 1 {
            // Return the vector and the name. Ignore IDs because the IDs used
            // in this method are not relevant outside it.
            return Ok(std::iter::zip(orbit, axis_names)
                .map(|((elem, vector), (_, name))| (elem, vector, name))
                .collect());
        }
        // If the multiplicity hasn't changed, then this iteration was useless
        // so undo it. I'm not sure whether this case is possible.
        if multiplicity == last_multiplicity {
            for name in axis_names.iter_values_mut() {
                name.pop();
            }
        }
        last_multiplicity = multiplicity;

        // Remove from consideration all named points within the
        // subspace, because they will not help us narrow down the axis
        // name.
        let old_ndim = subspace.ndim();
        for p in closest_points {
            subspace.add(&normalized_named_point_locations[p]);
        }
        ensure!(
            subspace.ndim() > old_ndim,
            "name_axis_orbit() failed to make progress",
        );
        candidates.retain(|&p| !subspace.contains(&normalized_named_point_locations[p]));
    }
}

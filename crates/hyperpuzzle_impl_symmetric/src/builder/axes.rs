use std::{
    collections::{HashMap, HashSet, VecDeque},
    num::NonZeroI32,
    sync::Arc,
};

use eyre::{OptionExt, Result, bail, eyre};
use hypergroup::{
    ConjugateCoset, Constraint, ConstraintSet, ConstraintSolver, CoxeterMatrix, GroupAction,
    GroupElementId, GroupError, IsometryGroup,
};
use hypermath::{APPROX, Float, Matrix, Point, Sign, Vector, VectorRef};
use hyperpuzzle_core::{
    Axis, AxisSystem, IndexOverflow, NameSpecBiMapBuilder, Orbit, PerAxis, TypedIndex,
    TypedIndexIter,
};
use hypuz_notation::{AxisLayersInfo, Str};
use hypuz_util::{FloatMinMaxByIteratorExt, FloatMinMaxIteratorExt};
use itertools::Itertools;
use smallvec::{SmallVec, smallvec};

use crate::{
    AxisOrbitSpec, StabilizableAxisSet, StabilizerFamily, UniqueMinimalClockwiseGenerator,
    names::NameBiMap,
};

/// Axis system of a puzzle under construction.
#[derive(Debug)]
pub(super) struct ProductPuzzleAxes {
    /// Grip group.
    pub group: IsometryGroup,
    /// Coxeter matrix for the grip group.
    pub coxeter_matrix: CoxeterMatrix,
    /// Action of the grip group on axes.
    pub action: GroupAction<Axis>,
    /// Vector for each axis.
    ///
    /// The vector is not necessarily normalized. Its magnitude determines the
    /// placement of twist gizmos in 3D and 4D. For a facet-turning puzzle, each
    /// axis vector will typically be scaled to match the distance of its
    /// corresponding facet.
    pub vectors: Arc<PerAxis<Vector>>,
    /// Axis orbits.
    pub orbits: Vec<AxisOrbit>,

    /// Pseudo-axes, defined as nonempty subsets of axes, each with a gizmo pole
    /// distance. Every axis has a pseudo-axis defined that contains it.
    ///
    /// Pseudo-axes are used to construct twist gizmos and named twists for 4D
    /// puzzles. Because they are not needed in higher dimensions, this list is
    /// made empty in 4D+.
    pub pseudo_axis_orbits: Vec<(StabilizableAxisSet, Float)>,
    /// Orbits of stabilizer twist families, along with their gizmo pole
    /// distances.
    ///
    /// These are the named twists in 4D puzzles. Because they are not needed in
    /// higher dimensions, this list is made empty in 5D+.
    pub stabilizer_twists: Vec<(StabilizerFamily, Float)>,
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
            action: GroupAction::trivial(),
            vectors: Arc::new(PerAxis::new()),
            orbits: vec![],

            pseudo_axis_orbits: vec![],
            stabilizer_twists: vec![],
        }
    }

    pub fn new(
        coxeter_matrix: CoxeterMatrix,
        group: IsometryGroup,
        axis_orbits: &[AxisOrbitSpec],
        pseudo_axis_orbits: &[(Vec<Str>, Float)],
        stabilizer_families: &[(Str, Vec<Str>, Float)],
        warn_fn: &mut impl FnMut(eyre::Report),
    ) -> Result<Self> {
        let generators = group.generator_motors();

        let mut all_names = NameBiMap::<Axis>::new();

        let mut vectors = PerAxis::new();
        let mut new_orbits = vec![];
        for orbit in axis_orbits {
            let mut names = vec![];
            let named_axis_vectors = orbit.named_axis_vectors(generators, |e| warn_fn(eyre!(e)));
            for (vector, name) in named_axis_vectors {
                vectors.push(vector.clone())?;
                names.push(name.clone());
                all_names.push(name.clone())?;
            }
            new_orbits.push(AxisOrbit {
                len: orbit.len(),
                prefix: hypuz_notation::family::SequentialLowercaseName(0),
                id_offset: 0,
                max_layer: orbit.layer_count().try_into()?,
                generator_sequences: Arc::new(
                    orbit
                        .names
                        .iter()
                        .map(|(abbr_gen_seq, _)| abbr_gen_seq.clone())
                        .collect(),
                ),
                names: Arc::new(names),
            });
        }

        // Shuffling group generators improves average word length, making some
        // group operations faster.
        let group = crate::shuffle_group_generators(&group, &mut rand::rng())?;

        let axis_points = vectors.map_ref(|_, v| Point(v.clone()));
        let action = group.action_on_points(&axis_points)?;

        let axis_from_name = |name: &Str| {
            all_names
                .name_to_id(name)
                .ok_or_else(|| eyre!("no axis named {name}"))
        };

        let pseudo_axis_orbits: Vec<(StabilizableAxisSet, Float)> = std::iter::chain(
            new_orbits
                .iter()
                .map(|orbit| orbit.first())
                .map(|ax| Ok((StabilizableAxisSet::new(smallvec![ax])?, vectors[ax].mag()))),
            pseudo_axis_orbits.iter().map(|(names, distance)| {
                eyre::Ok((
                    StabilizableAxisSet::new(names.iter().map(axis_from_name).try_collect()?)?,
                    *distance,
                ))
            }),
        )
        .try_collect()?;

        let stabilizer_twists = stabilizer_families
            .iter()
            .filter(|_| group.ndim() <= 4)
            .map(|(first, rest, distance)| {
                let primary = axis_from_name(first)?;
                let secondary =
                    StabilizableAxisSet::new(rest.iter().map(axis_from_name).try_collect()?)?;
                let stabilizer_family = StabilizerFamily { primary, secondary };
                eyre::Ok((stabilizer_family, *distance))
            })
            .try_collect()?;

        Ok(Self {
            group,
            coxeter_matrix,
            action,
            vectors: Arc::new(vectors),
            orbits: new_orbits,

            pseudo_axis_orbits,
            stabilizer_twists,
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

        let vectors = std::iter::chain(
            a.vectors
                .iter_values()
                .map(|v| crate::lift_vector_by_ndim(v, 0, a.ndim(), b.ndim())),
            b.vectors
                .iter_values()
                .map(|v| crate::lift_vector_by_ndim(v, a.ndim(), b.ndim(), 0)),
        )
        .collect();

        let orbits = std::iter::chain(
            a.orbits.iter().cloned().map(eyre::Ok),
            b.orbits.iter().cloned().map(|b_orbit| {
                Ok(b_orbit
                    .offset_ids_by(a.len())?
                    .offset_prefix_by(a.prefix_count()))
            }),
        )
        .try_collect()?;

        let lift_b_axis = |ax: Axis| Axis(ax.0 + a.len() as u16);
        let lift_b_axis_set = |ax_set: &StabilizableAxisSet| ax_set.offset_ids_by(a.len());

        let pseudo_axis_orbits = if ndim <= 4 {
            std::iter::chain(
                a.pseudo_axis_orbits.iter().cloned(),
                b.pseudo_axis_orbits
                    .iter()
                    .map(|(set, distance)| (lift_b_axis_set(set), *distance)),
            )
            .collect()
        } else {
            vec![]
        };

        let stabilizer_twists = if ndim <= 4 {
            itertools::chain!(
                a.stabilizer_twists.iter().cloned(),
                b.stabilizer_twists.iter().map(|(family, distance)| (
                    StabilizerFamily {
                        primary: lift_b_axis(family.primary),
                        secondary: lift_b_axis_set(&family.secondary),
                    },
                    *distance,
                )),
                itertools::iproduct!(&b.pseudo_axis_orbits, &a.orbits).map(
                    |((b_secondary, distance), a_orbit)| (
                        StabilizerFamily {
                            primary: a_orbit.first(),
                            secondary: lift_b_axis_set(b_secondary),
                        },
                        *distance,
                    )
                ),
                itertools::iproduct!(&a.pseudo_axis_orbits, &b.orbits).map(
                    |((a_secondary, distance), b_orbit)| (
                        StabilizerFamily {
                            primary: lift_b_axis(b_orbit.first()),
                            secondary: a_secondary.clone(),
                        },
                        *distance,
                    )
                ),
            )
            .collect()
        } else {
            vec![]
        };

        let group = IsometryGroup::product([&a.group, &b.group])?;
        let action = GroupAction::product([&a.action, &b.action])?;

        Ok(Self {
            coxeter_matrix: CoxeterMatrix::direct_product(&a.coxeter_matrix, &b.coxeter_matrix)?,
            group,
            action,
            vectors: Arc::new(vectors),
            orbits,

            pseudo_axis_orbits,
            stabilizer_twists,
        })
    }

    /// Returns the number of axes on the puzzle.
    pub fn len(&self) -> usize {
        self.vectors.len()
    }

    /// Returns the number of lowercase prefixes that are in use for axis sets.
    pub fn prefix_count(&self) -> u32 {
        self.orbits
            .iter()
            .map(|orbit| orbit.prefix.0 + 1)
            .max()
            .unwrap_or(0)
    }

    pub fn build_axis_system(&self) -> Result<AxisSystem> {
        let mut names = NameSpecBiMapBuilder::new();
        for orbit in &self.orbits {
            for (id, name) in std::iter::zip(orbit.axes(), &*orbit.names) {
                names.set(id, Some(format!("{}{}", orbit.prefix, name)))?;
            }
        }
        let names = Arc::new(names.build(self.len()).ok_or_eyre("missing axis name")?);

        let orbits = self
            .orbits
            .iter()
            .map(|orbit| Orbit {
                elements: Arc::new(orbit.axes().map(Some).collect()),
                generator_sequences: Arc::clone(&orbit.generator_sequences),
            })
            .collect();

        Ok(AxisSystem { names, orbits })
    }

    /// Returns the unit twist for each axis, or [`GroupElementId::IDENTITY`] if
    /// there is no unit twist for the axis.
    ///
    /// Only 3D puzzles have unit twists.
    pub fn build_3d_unit_twists(
        &self,
        solver: &mut ConstraintSolver<Axis>,
        warn_fn: &mut impl FnMut(eyre::Report),
    ) -> PerAxis<Option<UniqueMinimalClockwiseGenerator>> {
        let mut unit_twists = PerAxis::new_with_len(self.len());
        if self.ndim() == 3 {
            for orbit in &self.orbits {
                let first_axis = orbit.first();
                if let Some(stabilizer) = solver.solve(&ConstraintSet::from([[first_axis; 2]]))
                    && let Some(first_transform) = self.unit_twist_transform(
                        &stabilizer,
                        &[&self.vectors[first_axis]],
                        warn_fn,
                    )
                {
                    for axis in orbit.axes() {
                        if let Some(new_transform) = self.transfer_twist_transform(
                            solver,
                            (first_axis, first_transform.element),
                            axis,
                        ) {
                            unit_twists[axis] = Some(UniqueMinimalClockwiseGenerator {
                                element: new_transform,
                                order: first_transform.order,
                            });
                        }
                    }
                }
            }
        }
        unit_twists
    }

    pub fn build_4d_unit_twists(
        &self,
        solver: &mut ConstraintSolver<Axis>,
        warn_fn: &mut impl FnMut(eyre::Report),
    ) -> Vec<(StabilizerFamily, UniqueMinimalClockwiseGenerator)> {
        self.stabilizer_twists
            .iter()
            .filter_map(|(stabilizer_family, _distance)| {
                let constraint_set = match stabilizer_family.constraint_set() {
                    Ok(c) => c,
                    Err(e) => {
                        warn_fn(e);
                        return None;
                    }
                };
                let unit_twist = self.unit_twist_transform(
                    &solver.solve(&constraint_set)?,
                    &[
                        &self.vectors[stabilizer_family.primary],
                        &stabilizer_family.secondary.vector(&self.vectors),
                    ],
                    warn_fn,
                )?;
                Some((stabilizer_family.clone(), unit_twist))
            })
            .collect()
    }

    /// Returns the unique minimal clockwise generator for a coset, or `None` if
    /// there is not one.
    ///
    /// `stabilized_vectors` must be a list of vectors of length `ndim-2`, and
    /// is used to define "clockwise."
    fn unit_twist_transform(
        &self,
        stabilizer: &ConjugateCoset,
        stabilized_vectors: &[&Vector],
        warn_fn: &mut impl FnMut(eyre::Report),
    ) -> Option<UniqueMinimalClockwiseGenerator> {
        if stabilized_vectors.len() + 2 != self.ndim() as usize {
            warn_fn(eyre!("`stabilized_vectors` must have length ndim-2"));
            return None;
        }
        let nontrivial_rotations = stabilizer
            .elements()
            .into_iter()
            .filter(|&e| e != GroupElementId::IDENTITY)
            .filter(|&e| !self.group.is_reflection(e))
            .collect_vec();
        let order = NonZeroI32::new(nontrivial_rotations.len() as i32 + 1)?;
        let (mut min_group_element, min_rotation) = nontrivial_rotations
            .into_iter()
            .filter_map(|e| Some((e, self.group.motor(e).normalize()?)))
            .max_by_float_key(|(_e, m)| m.scalar().abs())?;
        let arbitrary_nonparallel_vector = Vector::unit(
            (0..self.ndim())
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
            min_group_element = self.group.inverse(min_group_element);
        }
        Some(UniqueMinimalClockwiseGenerator {
            element: min_group_element,
            order,
        })
    }

    fn transfer_twist_transform(
        &self,
        solver: &mut ConstraintSolver<Axis>,
        original: (Axis, GroupElementId),
        new_axis: Axis,
    ) -> Option<GroupElementId> {
        let (original_axis, original_transform) = original;

        let new_axis_deorbiter = solver
            .solve(&hypergroup::ConstraintSet::from([[
                original_axis,
                new_axis,
            ]]))?
            .lhs;
        let new_transform = self.group.conjugate(new_axis_deorbiter, original_transform);
        if self.group.is_reflection(new_axis_deorbiter) {
            Some(self.group.inverse(new_transform))
        } else {
            Some(new_transform)
        }
    }

    pub fn build_axis_layers(&self) -> PerAxis<AxisLayersInfo> {
        self.orbits
            .iter()
            .flat_map(|orbit| {
                orbit.axes().map(|_| AxisLayersInfo {
                    max_layer: orbit.max_layer,
                    allow_negatives: false, // TODO: allow negatives on some axes
                })
            })
            .collect()
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
}

impl AxisOrbit {
    /// Offsets all axis IDs by an additional amount.
    pub fn offset_ids_by(mut self, additional_id_offset: usize) -> Result<Self, IndexOverflow> {
        self.id_offset += additional_id_offset;
        Axis::try_iter_range(self.id_offset..self.id_offset + self.len)?; // check for overflow
        Ok(self)
    }

    /// Offsets the axis prefix by an additional amount.
    pub fn offset_prefix_by(mut self, additional_prefix_offset: u32) -> Self {
        self.prefix.0 += additional_prefix_offset;
        self
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

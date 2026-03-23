use std::sync::Arc;

use eyre::{OptionExt, Result, eyre};
use hypergroup::{ConstraintSolver, GroupAction, GroupElementId, IsometryGroup};
use hypermath::{Point, Sign, Vector, VectorRef};
use hyperpuzzle_core::{
    Axis, AxisSystem, IndexOverflow, NameSpecBiMapBuilder, Orbit, PerAxis, TypedIndex,
    TypedIndexIter,
};
use hypuz_notation::AxisLayersInfo;
use hypuz_util::FloatMinMaxByIteratorExt;
use itertools::Itertools;

use crate::{AxisOrbitSpec, names::NameBiMap};

/// Axis system of a puzzle under construction.
#[derive(Debug)]
pub(super) struct ProductPuzzleAxes {
    /// Grip group.
    pub group: IsometryGroup,
    /// Action of the grip group on axes.
    pub action: GroupAction<Axis>,
    /// Vector for each axis.
    pub vectors: Arc<PerAxis<Vector>>,
    /// Axis sets, each with a distinct lowercase Latin prefix.
    pub sets: Vec<AxisSet>,
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
            action: GroupAction::trivial(),
            vectors: Arc::new(PerAxis::new()),
            sets: vec![],
        }
    }

    pub fn new(
        symmetry: &IsometryGroup,
        axis_orbits: &[AxisOrbitSpec],
        warn_fn: &mut impl FnMut(eyre::Report),
    ) -> Result<Self> {
        let generators = symmetry.generator_motors();

        let mut vectors = PerAxis::new();
        let mut axis_names = NameBiMap::new();
        let mut new_orbits = vec![];
        for orbit in axis_orbits {
            new_orbits.push(AxisOrbit {
                len: orbit.len(),
                vector: orbit.initial_vector.clone(),
                max_layer: orbit.layer_count().try_into()?,
                generator_sequences: Arc::new(
                    orbit
                        .names
                        .iter()
                        .map(|(abbr_gen_seq, _)| abbr_gen_seq.clone())
                        .collect(),
                ),
            });
            let named_axis_vectors = orbit.named_axis_vectors(generators, |e| warn_fn(eyre!(e)));
            for (vector, name) in named_axis_vectors {
                vectors.push(vector.clone())?;
                axis_names.push(name.clone())?;
            }
        }

        let axis_set = AxisSet {
            ndim: symmetry.ndim(),
            len: vectors.len(),
            id_offset: 0,
            names: Arc::new(axis_names),
            orbits: new_orbits,
        };

        // Shuffling group generators improves average word length, making some
        // group operations faster.
        let symmetry = crate::shuffle_group_generators(&symmetry, &mut rand::rng());

        let axis_points = vectors.map_ref(|_, v| Point(v.clone()));
        let action = symmetry.action_on_points(&axis_points)?;

        Ok(Self {
            group: symmetry,
            action,
            vectors: Arc::new(vectors),
            sets: vec![axis_set],
        })
    }

    /// Returns the direct product of two axis systems.
    ///
    /// See [`super::ProductPuzzleBuilder::direct_product()`].
    pub fn direct_product(&self, rhs: &Self) -> Result<Self> {
        let a = self;
        let b = rhs;

        if (a.len() + b.len()).saturating_sub(1) > Axis::MAX_INDEX {
            return Err(IndexOverflow::new::<Axis>().into());
        }

        let a_axis_count = a.len();

        let vectors = std::iter::chain(
            a.vectors
                .iter_values()
                .map(|v| crate::lift_vector_by_ndim(v, 0, a.ndim(), b.ndim())),
            b.vectors
                .iter_values()
                .map(|v| crate::lift_vector_by_ndim(v, a.ndim(), b.ndim(), 0)),
        )
        .collect();

        let sets = std::iter::chain(
            a.sets
                .iter()
                .map(|a_axis_set| a_axis_set.lift_ndim(0, b.ndim())),
            b.sets.iter().map(|b_axis_set| {
                b_axis_set
                    .lift_ndim(a.ndim(), 0)
                    .offset_ids_by(a_axis_count)
            }),
        )
        .collect();

        Ok(Self {
            group: IsometryGroup::product([&a.group, &b.group])?,
            action: GroupAction::product([&a.action, &b.action])?,
            vectors: Arc::new(vectors),
            sets,
        })
    }

    /// Returns the number of axes on the puzzle.
    pub fn len(&self) -> usize {
        self.vectors.len()
    }

    /// Returns an iterator over axis orbits. Each item in the iterator
    /// contains:
    ///
    /// - First axis in the orbit
    /// - Range of axes in the orbit
    /// - Axis set, which is shared among several orbits
    /// - Axis orbit itself
    pub fn orbits(&self) -> impl Iterator<Item = AxisOrbitIterItem<'_>> {
        self.sets.iter().flat_map(|axis_set| {
            let mut next_orbit_start = axis_set.id_offset;
            axis_set.orbits.iter().map(move |axis_orbit| {
                let orbit_start = next_orbit_start;
                next_orbit_start += axis_orbit.len;
                let axis_range = Axis::iter_range(orbit_start..next_orbit_start);
                let first_axis = Axis::try_from_index(orbit_start).expect("unchecked overflow");
                AxisOrbitIterItem {
                    first_axis,
                    axis_range,
                    axis_set,
                    axis_orbit,
                }
            })
        })
    }

    pub fn build_axis_system(&self) -> Result<AxisSystem> {
        let mut names = NameSpecBiMapBuilder::new();
        for (i, axis_set) in self.sets.iter().enumerate() {
            let prefix = hypuz_notation::family::SequentialLowercaseName(i as _);
            for (id, name) in axis_set.names.id_to_name() {
                names.set(
                    Axis::try_from_index(axis_set.id_offset + id.to_index())?,
                    Some(format!("{prefix}{name}")),
                )?;
            }
        }
        let names = Arc::new(names.build(self.len()).ok_or_eyre("missing axis name")?);

        let orbits = self
            .orbits()
            .map(|orbit| Orbit {
                elements: Arc::new(orbit.axis_range.map(Some).collect()),
                generator_sequences: Arc::clone(&orbit.axis_orbit.generator_sequences),
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
    ) -> Result<PerAxis<(GroupElementId, i32)>> {
        let mut unit_twists = PerAxis::with_capacity(self.len());
        for orbit in self.orbits() {
            let first_axis = orbit.first_axis;
            let (first_transform, order) = if self.ndim() == 3 {
                self.axis_unit_twist_transform(solver, first_axis)
            } else {
                None
            }
            .unwrap_or((GroupElementId::IDENTITY, 0)); // sentinel indicating no unit twist
            for new_axis in orbit.axis_range {
                if order != 0
                    && let Some(new_transform) = self.transfer_twist_transform(
                        solver,
                        (first_axis, first_transform),
                        new_axis,
                    )
                {
                    unit_twists.push((new_transform, order))?;
                } else {
                    unit_twists.push((GroupElementId::IDENTITY, 0))?;
                }
            }
        }
        Ok(unit_twists)
    }

    /// Returns the unit twist and its order for an axis, or `None` if there is
    /// no unit twist for the axis.
    ///
    /// Only 3D puzzles have unit twists.
    fn axis_unit_twist_transform(
        &self,
        solver: &mut ConstraintSolver<Axis>,
        axis: Axis,
    ) -> Option<(GroupElementId, i32)> {
        let stabilizer =
            solver.solve(&hypergroup::ConstraintSet::from_iter([[axis, axis].into()]))?;
        let nontrivial_rotations = stabilizer
            .elements()
            .into_iter()
            .filter(|&e| e != GroupElementId::IDENTITY)
            .filter(|&e| !self.group.is_reflection(e))
            .collect_vec();
        let order = nontrivial_rotations.len() as i32 + 1;
        let (min_group_element, min_rotation) = nontrivial_rotations
            .into_iter()
            .filter_map(|e| Some((e, self.group.motor(e).normalize()?)))
            .max_by_float_key(|(_e, m)| m.scalar().abs())?;
        let axis_vector = &self.vectors[axis];
        let arbitrary_nonparallel_vector = Vector::unit(
            (0..3 as u8)
                .min_by_float_key(|&i| axis_vector[i].abs())
                .unwrap_or(0),
        );
        match Sign::from(
            arbitrary_nonparallel_vector
                .cross_product_3d(min_rotation.transform(&arbitrary_nonparallel_vector))
                .dot(axis_vector),
        ) {
            Sign::Pos => Some((self.group.inverse(min_group_element), order)),
            Sign::Neg => Some((min_group_element, order)),
        }
    }

    fn transfer_twist_transform(
        &self,
        solver: &mut ConstraintSolver<Axis>,
        original: (Axis, GroupElementId),
        new_axis: Axis,
    ) -> Option<GroupElementId> {
        let (original_axis, original_transform) = original;

        let new_axis_deorbiter = solver
            .solve(&hypergroup::ConstraintSet::from_iter([[
                original_axis,
                new_axis,
            ]
            .into()]))?
            .lhs;
        let new_transform = self.group.conjugate(new_axis_deorbiter, original_transform);
        if self.group.is_reflection(new_axis_deorbiter) {
            Some(self.group.inverse(new_transform))
        } else {
            Some(new_transform)
        }
    }

    pub fn build_axis_layers(&self) -> PerAxis<AxisLayersInfo> {
        self.orbits()
            .flat_map(|orbit| {
                orbit.axis_range.map(|_| AxisLayersInfo {
                    max_layer: orbit.axis_orbit.max_layer,
                    allow_negatives: false, // TODO: allow negatives on some axes
                })
            })
            .collect()
    }
}

/// Set of axes with a common lowercase Latin prefix.
///
/// This type is reference-counted and thus relatively cheap to clone.
#[derive(Debug)]
pub(super) struct AxisSet {
    /// Number of dimensions of the space containing the puzzle.
    pub ndim: u8,
    /// Number of axes in the set.
    pub len: usize,
    /// ID offset of the axes in the set.
    ///
    /// IDs within an set always count starting from 0, but the puzzle may have
    /// multiple sets and so puzzle-facing IDs for axes in this set must start
    /// counting from this offset.
    pub id_offset: usize,
    /// Axis names.
    pub names: Arc<NameBiMap<Axis>>,
    /// Axis orbits.
    pub orbits: Vec<AxisOrbit>,
}

impl AxisSet {
    /// Lifts the axis orbit into a higher dimension.
    ///
    /// - All axis vectors are lifted into a higher dimension.
    pub fn lift_ndim(&self, ndim_below: u8, ndim_above: u8) -> Self {
        Self {
            ndim: ndim_below + self.ndim + ndim_above,
            len: self.len,
            id_offset: self.id_offset,
            names: Arc::clone(&self.names),
            orbits: self
                .orbits
                .iter()
                .map(|axis_orbit| AxisOrbit {
                    len: axis_orbit.len,
                    vector: crate::lift_vector_by_ndim(
                        &axis_orbit.vector,
                        ndim_below,
                        self.ndim,
                        ndim_above,
                    ),
                    max_layer: axis_orbit.max_layer,
                    generator_sequences: Arc::clone(&axis_orbit.generator_sequences),
                })
                .collect(),
        }
    }

    /// Offsets all axis IDs by an additional amount.
    #[must_use]
    pub fn offset_ids_by(mut self, additional_offset: usize) -> Self {
        self.id_offset += additional_offset;
        self
    }
}

#[derive(Debug)]
pub struct AxisOrbit {
    /// Number of axes in the orbit.
    pub len: usize,
    /// Vector for the first axis.
    ///
    /// This vector is not necessarily normalized. Its magnitude determines the
    /// placement of twist gizmos in 4D. For a facet-turning puzzle, each axis
    /// vector will typically be scaled to match the distance of its
    /// corresponding facet.
    pub vector: Vector,
    /// Number of layers on each axis, which is equivalent to the maximum layer
    /// number.
    pub max_layer: u16,
    /// Generator sequence for each axis in the orbit.
    pub generator_sequences: Arc<Vec<hypergroup::AbbrGenSeq>>,
}

#[derive(Debug, Clone)]
pub struct AxisOrbitIterItem<'a> {
    pub first_axis: Axis,
    pub axis_range: TypedIndexIter<Axis>,
    pub axis_set: &'a AxisSet,
    pub axis_orbit: &'a AxisOrbit,
}

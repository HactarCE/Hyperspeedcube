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

use crate::AxisOrbitSpec;

/// Axis system of a puzzle under construction.
#[derive(Debug)]
pub(super) struct ProductPuzzleAxes {
    /// Grip group.
    pub group: IsometryGroup,
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
            orbits: vec![],
        }
    }

    pub fn new(
        symmetry: &IsometryGroup,
        axis_orbits: &[AxisOrbitSpec],
        warn_fn: &mut impl FnMut(eyre::Report),
    ) -> Result<Self> {
        let generators = symmetry.generator_motors();

        let mut vectors = PerAxis::new();
        let mut new_orbits = vec![];
        for orbit in axis_orbits {
            let mut names = vec![];
            let named_axis_vectors = orbit.named_axis_vectors(generators, |e| warn_fn(eyre!(e)));
            for (vector, name) in named_axis_vectors {
                vectors.push(vector.clone())?;
                names.push(name.clone());
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
        let symmetry = crate::shuffle_group_generators(symmetry, &mut rand::rng())?;

        let axis_points = vectors.map_ref(|_, v| Point(v.clone()));
        let action = symmetry.action_on_points(&axis_points)?;

        Ok(Self {
            group: symmetry,
            action,
            vectors: Arc::new(vectors),
            orbits: new_orbits,
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

        Ok(Self {
            group: IsometryGroup::product([&a.group, &b.group])?,
            action: GroupAction::product([&a.action, &b.action])?,
            vectors: Arc::new(vectors),
            orbits,
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
    ) -> Result<PerAxis<(GroupElementId, i32)>> {
        let mut unit_twists = PerAxis::with_capacity(self.len());
        for orbit in &self.orbits {
            let (first_transform, order) = if self.ndim() == 3 {
                self.axis_unit_twist_transform(solver, orbit.first())
            } else {
                None
            }
            .unwrap_or((GroupElementId::IDENTITY, 0)); // sentinel indicating no unit twist
            for new_axis in orbit.axes() {
                if order != 0
                    && let Some(new_transform) = self.transfer_twist_transform(
                        solver,
                        (orbit.first(), first_transform),
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
            (0..3)
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

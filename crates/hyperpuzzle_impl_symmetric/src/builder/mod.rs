//! Symmetric Euclidean puzzle simulation backend and Hyperpuzzlescript API for
//! Hyperspeedcube.

use std::ops::Range;
use std::sync::{Arc, Weak};

use eyre::{OptionExt, Result, eyre};
use hypergroup::{ConstraintSolver, GroupElementId};
use hypermath::prelude::*;
use hyperpuzzle_core::catalog::{BuildCtx, BuildTask};
use hyperpuzzle_core::group::{GroupAction, IsometryGroup};
use hyperpuzzle_core::prelude::*;
use hyperpuzzle_impl_nd_euclid::{NdEuclidPuzzleGeometry, NdEuclidPuzzleUiData};

mod axes;
mod from_space;
mod gizmos;
mod shape;

use hypuz_util::FloatMinMaxByIteratorExt;
use itertools::Itertools;
use parking_lot::Mutex;
use rand::seq::IndexedRandom;
use shape::{PieceData, PieceFacetData, ProductPuzzleShape, StickerData, SurfaceData};

use crate::names::NameBiMap;
use crate::{
    FactorPuzzleSpec, ProductPuzzleSpec, ProductPuzzleState, SymmetricTwistSystemEngineData,
};
use axes::{AxisOrbit, AxisSet};

#[derive(Debug)]
pub struct ProductPuzzleBuilder {
    shape: ProductPuzzleShape,

    axis_group: IsometryGroup,
    axis_group_action: GroupAction<Axis>,
    axis_vectors: PerAxis<Vector>,
    axis_sets: Vec<AxisSet>,
    // /// Twist gizmos to generate, which will be expanded by the symmetry of the
    // /// puzzle.
    // twist_gizmo_seeds: Vec<TwistGizmoBuilder>,
}

impl ProductPuzzleBuilder {
    /// Returns the number of the dimensions of the puzzle.
    pub fn ndim(&self) -> u8 {
        self.shape.ndim
    }

    /// Returns the number of axes on the puzzle.
    pub fn axis_count(&self) -> usize {
        self.axis_sets.iter().map(|axis_set| axis_set.len).sum()
    }

    /// Constructs the empty puzzle, which is the identity of the direct
    /// product.
    pub fn direct_product_identity() -> Self {
        ProductPuzzleBuilder {
            shape: ProductPuzzleShape::direct_product_identity(),

            axis_group: IsometryGroup::trivial(),
            axis_group_action: GroupAction::trivial(),
            axis_vectors: PerAxis::new(),
            axis_sets: vec![],
        }
    }

    /// Constructs a symmetric puzzle.
    pub fn new(
        product_puzzle_spec: &ProductPuzzleSpec,
        warn_fn: &mut impl FnMut(eyre::Report),
    ) -> Result<Self> {
        product_puzzle_spec
            .factors
            .iter()
            .map(|factor_spec| Self::new_factor(factor_spec, warn_fn))
            .try_fold(ProductPuzzleBuilder::direct_product_identity(), |a, b| {
                a.direct_product(&b?)
            })
    }

    fn new_factor(spec: &FactorPuzzleSpec, warn_fn: &mut impl FnMut(eyre::Report)) -> Result<Self> {
        let generators = spec.symmetry.generator_motors();

        let mut shape_builder =
            from_space::PuzzleShapeBuilder::new(spec.ndim(), spec.axis_count())?;
        // TODO: color orbits (dev data)
        for orbit in &spec.facet_orbits {
            let named_facet_poles = orbit.named_facet_poles(generators, |e| warn_fn(eyre!(e)));
            for (pole, name) in named_facet_poles {
                let plane = Hyperplane::from_pole(pole).ok_or_eyre("bad hyperplane")?;
                shape_builder.carve(plane, name)?;
            }
        }

        shape_builder.set_surface_centroids_from_stickers_of_single_piece(Piece(0))?;

        let mut axis_vectors = PerAxis::new();
        let mut axis_names = NameBiMap::new();
        let mut axis_orbits = vec![];
        for orbit in &spec.axis_orbits {
            axis_orbits.push(AxisOrbit {
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
                let axis = axis_vectors.push(vector.clone())?;
                axis_names.push(name.clone())?;
                for (layer, cut_distance) in orbit.layer_cut_distances() {
                    shape_builder.slice(axis, &vector, cut_distance, layer)?;
                }
            }
        }

        let shape = shape_builder.into_product_puzzle_shape()?;

        let axis_set = AxisSet {
            ndim: spec.ndim(),
            len: axis_vectors.len(),
            id_offset: 0,
            names: Arc::new(axis_names),
            orbits: axis_orbits,
        };

        // Shuffling group generators improves average word length, making some
        // group operations faster.
        let symmetry = shuffle_group_generators(&spec.symmetry, &mut rand::rng());

        let axis_points = axis_vectors.map_ref(|_, v| Point(v.clone()));
        let axis_group_action = symmetry.action_on_points(&axis_points)?;

        Ok(Self {
            shape,

            axis_group: symmetry,
            axis_group_action,
            axis_vectors,
            axis_sets: vec![axis_set],
        })
    }

    /// Returns an iterator over axis orbits, each paired with the ID range of
    /// the axes in the orbit. The ID range is never empty.
    fn axis_orbits(
        &self,
    ) -> Result<impl Iterator<Item = (Range<Axis>, &AxisSet, &AxisOrbit)>, IndexOverflow> {
        if self.axis_count() > Axis::MAX_INDEX {
            return Err(IndexOverflow::new::<Axis>());
        }
        Ok(self.axis_sets.iter().flat_map(|axis_set| {
            let mut first_axis_in_next_orbit = Axis(axis_set.id_offset as u16);
            axis_set.orbits.iter().map(move |axis_orbit| {
                let first_axis_in_orbit = first_axis_in_next_orbit;
                first_axis_in_next_orbit.0 += axis_orbit.len as u16;
                let axis_range = first_axis_in_orbit..first_axis_in_next_orbit;
                (axis_range, axis_set, axis_orbit)
            })
        }))
    }

    /// Returns the direct product of two puzzles.
    ///
    /// The direct product of two puzzles `a` and `b` will have dimension
    /// `a.ndim() + b.ndim()`, with puzzle `a` occupying the lower dimensions
    /// and puzzle `b` occupying the higher dimensions.
    pub fn direct_product(&self, rhs: &Self) -> Result<Self> {
        let a = self;
        let b = rhs;

        let a_axis_count = a.axis_count();

        let axis_sets = std::iter::chain(
            a.axis_sets
                .iter()
                .map(|a_axis_set| a_axis_set.lift_ndim(0, b.ndim())),
            b.axis_sets.iter().map(|b_axis_set| {
                b_axis_set
                    .lift_ndim(a.ndim(), 0)
                    .offset_ids_by(a_axis_count)
            }),
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

        Ok(ProductPuzzleBuilder {
            shape: ProductPuzzleShape::direct_product(&a.shape, &b.shape)?,

            axis_group: IsometryGroup::product([&a.axis_group, &b.axis_group])?,
            axis_group_action: GroupAction::product([&a.axis_group_action, &b.axis_group_action])?,
            axis_sets,
            axis_vectors,
        })
    }

    /// Constructs the final puzzle.
    pub fn build(
        &self,
        build_ctx: Option<&BuildCtx>,
        warn_fn: &mut impl FnMut(eyre::Report),
    ) -> Result<Arc<Puzzle>> {
        if let Some(build_ctx) = build_ctx {
            build_ctx.progress.lock().task = BuildTask::BuildingPuzzle;
        }

        let ndim = self.ndim();
        let piece_count = self.shape.pieces.len();

        let (pieces, stickers) = self.shape.build_piece_and_stickers()?;

        let colors = Arc::new(self.shape.build_colors(warn_fn)?);

        let (piece_types, piece_type_hierarchy, piece_type_masks) =
            self.shape.build_piece_types(warn_fn)?;

        let mut mesh = self.shape.build_mesh()?;

        let axes = Arc::new(AxisSystem {
            names: {
                let mut axis_names = NameSpecBiMapBuilder::new();
                for (i, axis_set) in self.axis_sets.iter().enumerate() {
                    let prefix = hypuz_notation::family::SequentialLowercaseName(i as _);
                    for (id, name) in axis_set.names.id_to_name() {
                        axis_names.set(
                            Axis::try_from_index(axis_set.id_offset + id.to_index())?,
                            Some(format!("{prefix}{name}")),
                        )?;
                    }
                }
                Arc::new(
                    axis_names
                        .build(self.axis_count())
                        .ok_or_eyre("missing axis name")?,
                )
            },
            orbits: {
                let mut cumulative_axis_count = 0;
                self.axis_sets
                    .iter()
                    .flat_map(|axis_set| {
                        axis_set.orbits.iter().map(move |axis_orbit| {
                            let elements = Arc::new(
                                (0..axis_orbit.len)
                                    .map(|i| {
                                        Axis::try_from_index(cumulative_axis_count + i).map(Some)
                                    })
                                    .try_collect()?,
                            );
                            cumulative_axis_count += axis_orbit.len;
                            eyre::Ok(Orbit {
                                elements,
                                generator_sequences: Arc::clone(&axis_orbit.generator_sequences),
                            })
                        })
                    })
                    .try_collect()?
            },
        });
        let axis_vectors = Arc::new(self.axis_vectors.clone());
        let mut axis_constraint_solver =
            hypergroup::ConstraintSolver::new(self.axis_group_action.clone());
        let mut axis_unit_twists = PerAxis::new();
        for (axis_range, _axis_set, _axis_orbit) in self.axis_orbits()? {
            let first_axis = axis_range.start;
            let unit_twist_transform = if ndim == 3 {
                axis_unit_twist_transform(
                    &self.axis_group,
                    &mut axis_constraint_solver,
                    first_axis,
                    &self.axis_vectors[first_axis],
                )
            } else {
                None
            };
            for axis in (axis_range.start.0..axis_range.end.0).map(Axis) {
                axis_unit_twists.push(
                    unit_twist_transform
                        .and_then(|transform| {
                            transfer_twist_transform(
                                &self.axis_group,
                                &mut axis_constraint_solver,
                                (first_axis, transform),
                                axis,
                            )
                        })
                        .unwrap_or(GroupElementId::IDENTITY), // sentinel indicating no unit twist
                )?;
            }
        }
        let mut twists = TwistSystem::new_empty(&axes);
        let twist_system_engine_data = Arc::new(SymmetricTwistSystemEngineData {
            axes,
            axis_vectors,
            group: self.axis_group.clone(),
            group_action: self.axis_group_action.clone(),
            constraint_solver: Arc::new(Mutex::new(axis_constraint_solver)),

            axis_unit_twists: Arc::new(axis_unit_twists),
        });
        twists.engine_data = (*twist_system_engine_data).clone().into();
        let twists = Arc::new(twists);

        let axis_layers: PerAxis<AxisLayersInfo> = self
            .axis_orbits()?
            .flat_map(|(_axis_range, _axis_set, axis_orbit)| {
                std::iter::repeat_n(
                    AxisLayersInfo {
                        max_layer: axis_orbit.max_layer,
                        allow_negatives: false, // TODO: allow negatives on some axes
                    },
                    axis_orbit.len,
                )
            })
            .collect();

        let grip_signatures = Arc::new(
            self.shape
                .pieces
                .map_ref(|_, piece| piece.grip_signature.clone()),
        );

        let axes_with_twists: Vec<Axis> = self
            .axis_orbits()?
            .filter(|(axis_range, _axis_set, axis_orbit)| {
                axis_orbit.max_layer > 0
                    && twist_system_engine_data.axis_has_twists(axis_range.start)
            })
            .flat_map(|(axis_range, _axis_set, _axis_orbit)| {
                (axis_range.start.0..axis_range.end.0).map(Axis)
            })
            .collect();

        let mut mesh = self.shape.build_mesh()?;

        let mut gizmo_twists = PerGizmoFace::new();
        if ndim == 3 {
            let mut gizmo_faces = vec![];
            for (axis_range, _axis_set, _axis_orbit) in self.axis_orbits()? {
                for axis in (axis_range.start.0..axis_range.end.0).map(Axis) {
                    let twist_transform =
                        hypuz_notation::Transform::new(&twists.axes.names[axis], None);
                    gizmo_faces.push((self.axis_vectors[axis].clone(), axis, twist_transform));
                }
            }
            gizmos::build_3d_gizmo(&mut mesh, &gizmo_faces, &mut gizmo_twists)?;
        }

        let geom = Arc::new(NdEuclidPuzzleGeometry {
            vertex_coordinates: vec![],
            piece_vertex_sets: PerPiece::new_with_len(piece_count),
            piece_centroids: self
                .shape
                .pieces
                .map_ref(|_, piece_geometries| piece_geometries.polytope.centroid.center()),

            planes: vec![Hyperplane::new(vector![1.0], 0.0).unwrap()],
            sticker_planes: stickers.map_ref(|_, _| 0),

            mesh,

            axis_vectors: Arc::new(PerAxis::new()),
            axis_layer_depths: PerAxis::new(),
            twist_transforms: Arc::new(PerTwist::new()),

            gizmo_twists,
        });
        let ui_data = NdEuclidPuzzleUiData::new_dyn(&geom);

        let random_move = Box::new({
            let twist_system_engine_data = Arc::clone(&twist_system_engine_data);
            let axis_layers = Arc::new(axis_layers.clone());
            move |rng: &mut dyn rand::Rng| {
                let axis = *axes_with_twists.choose(rng)?;
                let layers =
                    hyperpuzzle_core::util::random_layer_mask(rng, axis_layers[axis].max_layer)?;
                let family = &twist_system_engine_data.axes.names[axis];
                let constraints =
                    Some(twist_system_engine_data.random_constraints_on_axis(rng, axis)?)
                        .filter(|c| !c.constraints.is_empty());
                Some(Move::new(layers, family, constraints, 1))
            }
        });

        Ok(Arc::new_cyclic(|this| Puzzle {
            this: Weak::clone(this),
            meta: Arc::new(PuzzleListMetadata {
                id: "symmetric_puzzle_test".to_string(),
                version: Version {
                    major: 0,
                    minor: 0,
                    patch: 1,
                },
                name: "Symmetric Puzzle Test".to_string(),
                aliases: vec![],
                tags: TagSet::new(),
            }),
            view_prefs_set: Some(PuzzleViewPreferencesSet::Perspective(match ndim {
                ..=3 => PerspectiveDim::Dim3D,
                4.. => PerspectiveDim::Dim4D,
            })),
            pieces,
            stickers,
            piece_types,
            piece_type_hierarchy,
            piece_type_masks,
            colors,
            can_scramble: false,
            full_scramble_length: hyperpuzzle_core::FULL_SCRAMBLE_LENGTH,
            axis_layers,
            twists,
            ui_data,
            new: Box::new({
                move |ty| {
                    ProductPuzzleState {
                        ty,
                        twists: Arc::clone(&twist_system_engine_data),
                        piece_grip_signatures: Arc::clone(&grip_signatures),
                        piece_attitudes: PerPiece::new_with_len(piece_count),
                    }
                    .into()
                }
            }),
            random_move,
        }))
    }
}

fn shuffle_group_generators(group: &IsometryGroup, mut rng: impl rand::Rng) -> IsometryGroup {
    use rand::RngExt;

    const SHUFFLE_ITERATIONS: usize = 100;

    if group.generators().len() < 2 {
        return group.clone();
    }

    // TODO: add more generators, especially for polygons
    let mut generators = group.generator_motors().to_vec();
    for _ in 0..SHUFFLE_ITERATIONS {
        let i = rng.random_range(0..generators.len());
        let mut j = rng.random_range(0..generators.len() - 1);
        if j >= i {
            j += 1;
        }
        generators[i] = &generators[i] * &generators[j];
    }
    IsometryGroup::from_generators("", hypergroup::PerGenerator::from(generators)).unwrap()
}

fn axis_unit_twist_transform(
    group: &IsometryGroup,
    solver: &mut ConstraintSolver<Axis>,
    axis: Axis,
    axis_vector: &Vector,
) -> Option<GroupElementId> {
    if let Some(stabilizer) =
        solver.solve(&hypergroup::ConstraintSet::from_iter([[axis, axis].into()]))
        && let stabilizer_elements = stabilizer.elements().into_iter()
        && let nontrivial_rotations = stabilizer_elements
            .filter(|&e| e != GroupElementId::IDENTITY)
            .filter(|&e| !group.is_reflection(e))
        && let Some((min_group_element, min_rotation)) = nontrivial_rotations
            .filter_map(|e| Some((e, group.motor(e).normalize()?)))
            .max_by_float_key(|(_e, m)| m.scalar().abs())
    {
        let arbitrary_perpendicular_vector = Vector::unit(
            (0..3 as u8)
                .min_by_float_key(|&i| axis_vector[i].abs())
                .unwrap_or(0),
        );

        match Sign::from(
            arbitrary_perpendicular_vector
                .cross_product_3d(min_rotation.transform(&arbitrary_perpendicular_vector))
                .dot(axis_vector),
        ) {
            Sign::Pos => Some(group.inverse(min_group_element)),
            Sign::Neg => Some(min_group_element),
        }
    } else {
        None
    }
}

fn transfer_twist_transform(
    group: &IsometryGroup,
    solver: &mut ConstraintSolver<Axis>,
    original: (Axis, GroupElementId),
    new_axis: Axis,
) -> Option<GroupElementId> {
    let (original_axis, original_twist_transform) = original;

    let new_axis_deorbiter = solver
        .solve(&hypergroup::ConstraintSet::from_iter([[
            original_axis,
            new_axis,
        ]
        .into()]))?
        .lhs;
    let new_transform = group.conjugate(new_axis_deorbiter, original_twist_transform);
    if group.is_reflection(new_axis_deorbiter) {
        Some(group.inverse(new_transform))
    } else {
        Some(new_transform)
    }
}

//! Symmetric Euclidean puzzle simulation backend and Hyperpuzzlescript API for
//! Hyperspeedcube.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Weak};

use eyre::{OptionExt, Result, eyre};
use hypergroup::GroupError;
use hypermath::prelude::*;
use hyperpuzzle_core::catalog::{BuildCtx, BuildTask};
use hyperpuzzle_core::prelude::*;
use hyperpuzzle_impl_nd_euclid::{NdEuclidPuzzleGeometry, NdEuclidPuzzleUiData};

mod axes;
mod from_space;
mod gizmos;
mod shape;

use axes::ProductPuzzleAxes;
use itertools::Itertools;
use parking_lot::Mutex;
use rand::RngExt;
use rand::seq::IndexedRandom;
use shape::{PieceData, PieceFacetData, ProductPuzzleShape, StickerData, SurfaceData};

use crate::{
    FactorPuzzleSpec, ProductPuzzleSpec, ProductPuzzleState, StabilizerFamily,
    SymmetricTwistSystemEngineData,
};

#[derive(Debug)]
pub struct ProductPuzzleBuilder {
    factor_ids: Vec<CatalogId>,
    shape: ProductPuzzleShape,
    axes: ProductPuzzleAxes,
}

impl ProductPuzzleBuilder {
    /// Returns the number of the dimensions of the puzzle.
    pub fn ndim(&self) -> u8 {
        debug_assert_eq!(self.shape.ndim(), self.axes.ndim());
        self.shape.ndim()
    }

    /// Constructs the empty puzzle, which is the identity of the direct
    /// product.
    pub fn direct_product_identity() -> Self {
        ProductPuzzleBuilder {
            factor_ids: vec![],
            shape: ProductPuzzleShape::direct_product_identity(),
            axes: ProductPuzzleAxes::direct_product_identity(),
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
        let group = spec.coxeter_matrix.isometry_group()?;
        let generators = group.generator_motors();

        let mut shape_builder = from_space::PuzzleShapeFactorBuilder::new(
            spec.shape_id.clone(),
            spec.coxeter_matrix.clone(),
            group.clone(),
        )?;

        // TODO: color orbits (dev data)

        // Carve facets
        for orbit in &spec.facet_orbits {
            let named_facet_poles = orbit.named_facet_poles(generators, |e| warn_fn(eyre!(e)));
            for (pole, name) in named_facet_poles {
                let plane = Hyperplane::from_pole(pole).ok_or_eyre("bad hyperplane")?;
                let color = shape_builder.add_color(name.clone())?;
                shape_builder.carve(plane, color)?;
            }
        }
        shape_builder.set_surface_centroids_from_stickers_of_single_piece(Piece(0))?;

        let axes = ProductPuzzleAxes::new(
            spec.coxeter_matrix.clone(),
            group.clone(),
            &spec.axis_orbits,
            &spec.pseudo_axis_orbits,
            &spec.axis_pairs,
            warn_fn,
        )?;

        // Slice axes
        for (orbit, axis_orbit_spec) in std::iter::zip(&axes.orbits, &spec.axis_orbits) {
            for axis in orbit.axes() {
                for &cut_distance in &axis_orbit_spec.cut_distances {
                    let plane = Hyperplane::new(&axes.vectors[axis], cut_distance)
                        .ok_or_eyre("bad axis vector")?;
                    shape_builder.slice(plane)?;
                }
            }
        }

        let mut shape = shape_builder.into_product_puzzle_shape()?;

        // Add grip signatures
        for (_, piece_data) in &mut shape.pieces {
            piece_data.grip_signature = PerAxis::new_with_len(axes.len());
            for (orbit, axis_orbit_spec) in std::iter::zip(&axes.orbits, &spec.axis_orbits) {
                for axis in orbit.axes() {
                    if let Some((min_height, max_height)) =
                        piece_data.polytope.height_on_axis(&axes.vectors[axis])
                    {
                        piece_data.grip_signature[axis] =
                            axis_orbit_spec.layer_range_for_distance_range(max_height, min_height);
                    }
                }
            }
        }

        Ok(Self {
            factor_ids: vec![spec.puzzle_id.clone()],
            shape,
            axes,
        })
    }

    /// Returns the direct product of two puzzles.
    ///
    /// The direct product of two puzzles `a` and `b` will have dimension
    /// `a.ndim() + b.ndim()`, with puzzle `a` occupying the lower dimensions
    /// and puzzle `b` occupying the higher dimensions.
    pub fn direct_product(&self, rhs: &Self) -> Result<Self> {
        Ok(ProductPuzzleBuilder {
            factor_ids: std::iter::chain(&self.factor_ids, &rhs.factor_ids)
                .cloned()
                .collect(),
            shape: self.shape.direct_product(&rhs.shape)?,
            axes: self.axes.direct_product(&rhs.axes)?,
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

        let id = crate::product_id(&self.factor_ids);

        let (pieces, stickers) = self.shape.build_piece_and_stickers()?;

        let colors = Arc::new(self.shape.build_colors(warn_fn)?);

        let (piece_types, piece_type_hierarchy, piece_type_masks) =
            self.shape.build_piece_types(warn_fn)?;

        let axes = Arc::new(self.axes.build_axis_system()?);
        let mut axis_constraint_solver =
            hypergroup::ConstraintSolver::new(self.axes.action.clone());
        let mut twists = TwistSystem::new_empty(&axes);
        let axis_unit_twists = self
            .axes
            .build_3d_unit_twists(&mut axis_constraint_solver, warn_fn);
        let stabilizer_twists = self
            .axes
            .build_4d_unit_twists(&mut axis_constraint_solver, warn_fn);
        let twist_system_engine_data = Arc::new(SymmetricTwistSystemEngineData {
            axes,
            axis_vectors: Arc::clone(&self.axes.vectors),
            group: self.axes.group.clone(),
            group_action: self.axes.action.clone(),
            constraint_solver: Arc::new(Mutex::new(axis_constraint_solver)),

            axis_unit_twists: Arc::new(axis_unit_twists),

            stabilizer_twists: Arc::new(stabilizer_twists),
        });
        twists.engine_data = (*twist_system_engine_data).clone().into();
        let twists = Arc::new(twists);

        let axis_layers: PerAxis<AxisLayersInfo> = self.axes.build_axis_layers();

        let grip_signatures = Arc::new(self.shape.build_grip_signatures());

        let axes_with_twists: Vec<Axis> = self
            .axes
            .orbits
            .iter()
            .filter(|orbit| {
                orbit.max_layer > 0 && twist_system_engine_data.axis_has_twists(orbit.first())
            })
            .flat_map(|orbit| orbit.axes())
            .collect();

        let mut mesh = self.shape.build_mesh()?;

        let mut gizmo_twists = PerGizmoFace::new();
        if ndim == 3 {
            gizmos::build_3d_gizmo(&mut mesh, &mut gizmo_twists, &self.axes, &twists.axes.names)?;
        } else if ndim == 4 {
            gizmos::build_4d_gizmo(
                &mut mesh,
                &mut gizmo_twists,
                &mut twist_system_engine_data.constraint_solver.lock(),
                &self.axes,
                &twists.axes.names,
                warn_fn,
            )?;
        }

        let (planes, sticker_planes) = self.shape.build_sticker_planes();

        let geom = Arc::new(NdEuclidPuzzleGeometry {
            vertex_coordinates: vec![],
            piece_vertex_sets: PerPiece::new_with_len(piece_count),
            piece_centroids: self
                .shape
                .pieces
                .map_ref(|_, piece_geometries| piece_geometries.polytope.centroid.center()),

            planes,
            sticker_planes,

            mesh,

            axis_vectors: Arc::new(PerAxis::new()), // TODO: is this needed?
            axis_layer_depths: PerAxis::new(),      // TODO: is this needed?
            twist_transforms: Arc::new(PerTwist::new()),

            gizmo_twists,
        });
        let ui_data = NdEuclidPuzzleUiData::new_dyn(&geom);

        let random_move = Box::new({
            let twist_system_engine_data = Arc::clone(&twist_system_engine_data);
            let axis_layers = Arc::new(axis_layers.clone());
            move |rng: &mut dyn rand::Rng| {
                let axis = *axes_with_twists.choose(rng)?;
                // TODO: avoid total layer mask when that covers all pieces
                let layers =
                    hyperpuzzle_core::util::random_layer_mask(rng, axis_layers[axis].max_layer)?;
                let family = &twist_system_engine_data.axes.names[axis];
                if let Some(unit_twist) = twist_system_engine_data.axis_unit_twists[axis] {
                    let order = unit_twist.order.get();
                    let mut multiplier = rng.random_range(1..order); // guaranteed nonempty
                    if multiplier * 2 > order {
                        multiplier -= order;
                    }
                    Some(Move::new(layers, family, None, multiplier))
                } else {
                    let constraints =
                        Some(twist_system_engine_data.random_constraints_on_axis(rng, axis)?)
                            .filter(|c| !c.constraints.is_empty());
                    Some(Move::new(layers, family, constraints, 1))
                }
            }
        });

        Ok(Arc::new_cyclic(|this| Puzzle {
            this: Weak::clone(this),
            meta: Arc::new(PuzzleListMetadata {
                id,
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

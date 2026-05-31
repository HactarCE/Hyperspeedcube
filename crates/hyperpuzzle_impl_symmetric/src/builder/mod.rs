//! Symmetric Euclidean puzzle simulation backend and Hyperpuzzlescript API for
//! Hyperspeedcube.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Weak};

use eyre::{Context, OptionExt, Result, eyre};
use hypergroup::{
    CoxeterMatrix, GroupElementId, GroupError, SubgroupAction, SubgroupConstraintSolver,
};
use hypermath::prelude::*;
use hyperpuzzle_core::catalog::{BuildCtx, BuildTask, GeneratorOutput};
use hyperpuzzle_core::prelude::*;
use hyperpuzzle_core::util::MaybeAdHoc;
use hyperpuzzle_core::{Component, ComponentList};
use hyperpuzzle_impl_nd_euclid::{NdEuclidAxisVectors, NdEuclidPuzzleGeometry};

mod colors;
mod from_space;
mod gizmos;
mod shape;
mod twists;

pub(crate) use colors::ColorSystemDisjointUnion;
use itertools::Itertools;
use parking_lot::Mutex;
use rand::RngExt;
use rand::seq::IndexedRandom;
use shape::{PieceData, PieceFacetData, ProductPuzzleShape, StickerData, SurfaceData};
pub(crate) use twists::TwistSystemProduct;

use crate::spec::FacetOrbitSpec;
use crate::{
    CutDistances, FactorPuzzleSpec, ProductPuzzleSpec, ProductPuzzleState, StabilizerFamily,
    SymmetricTwistSystemAxisOrbit, SymmetricTwistSystemComponent,
};

#[derive(Debug)]
pub struct PuzzleProduct {
    factor_ids: Vec<CatalogId>,
    factor_names: Vec<String>,
    shape: ProductPuzzleShape,
    colors: MaybeAdHoc<ColorSystem, ColorSystemDisjointUnion>,
    twists: MaybeAdHoc<TwistSystem, TwistSystemProduct>,
    axis_layers_per_orbit: Vec<AxisLayersInfo>,
}

impl PuzzleProduct {
    /// Returns the number of the dimensions of the puzzle.
    pub fn ndim(&self) -> u8 {
        if let Ok(ad_hoc_twists) = self.twists.as_ad_hoc() {
            debug_assert_eq!(self.shape.ndim(), ad_hoc_twists.ndim());
        }
        self.shape.ndim()
    }

    /// Constructs the empty puzzle, which is the identity of the direct
    /// product.
    pub fn direct_product_identity() -> Self {
        PuzzleProduct {
            factor_ids: vec![],
            factor_names: vec![],
            shape: ProductPuzzleShape::direct_product_identity(),
            colors: MaybeAdHoc::AdHoc(ColorSystemDisjointUnion::disjoint_union_identity()),
            twists: MaybeAdHoc::AdHoc(TwistSystemProduct::direct_product_identity()),
            axis_layers_per_orbit: vec![],
        }
    }

    /// Constructs a factor puzzle builder.
    ///
    /// Note that `axis_orbit_cut_distances` must be sorted from outermost
    /// (greatest) to innermost (least).
    pub fn new_factor(
        id: CatalogId,
        name: String,
        coxeter_matrix: CoxeterMatrix,
        facet_orbits: &[FacetOrbitSpec],
        colors: MaybeAdHoc<ColorSystem, ColorSystemDisjointUnion>,
        twists: MaybeAdHoc<TwistSystem, TwistSystemProduct>,
        axis_orbit_cut_distances: &[CutDistances],
        warn_fn: &mut impl FnMut(eyre::Report),
    ) -> Result<Self> {
        let group = coxeter_matrix.isometry_group()?;
        let generators = group.generator_motors();

        let mut shape_builder = from_space::PuzzleShapeFactorBuilder::new(coxeter_matrix, group)?;

        // TODO: color orbits (dev data)

        // Carve facets
        for orbit in facet_orbits {
            for (pole, name, _gen_seq) in &orbit.named_facet_poles {
                let plane = Hyperplane::from_pole(pole).ok_or_eyre("bad hyperplane")?;
                let color = shape_builder.add_color(name.clone())?;
                shape_builder.carve(plane, color)?;
            }
        }
        shape_builder.set_surface_centroids_from_stickers_of_single_piece(Piece(0))?;

        let product_twists = twists.as_ad_hoc_or_component()?;

        // Slice axes
        for (orbit, cut_distances) in
            std::iter::zip(&product_twists.axis_orbits, axis_orbit_cut_distances)
        {
            for axis in orbit.axes() {
                for &cut_distance in &cut_distances.0 {
                    let plane = Hyperplane::new(&product_twists.axis_vectors[axis], cut_distance)
                        .ok_or_eyre("bad axis vector")?;
                    shape_builder.slice(plane)?;
                }
            }
        }

        let mut shape = shape_builder.into_product_puzzle_shape()?;

        // Add grip signatures
        for (_, piece_data) in &mut shape.pieces {
            piece_data.grip_signature = PerAxis::new_with_len(product_twists.len());
            for (orbit, cut_distances) in
                std::iter::zip(&product_twists.axis_orbits, axis_orbit_cut_distances)
            {
                let recip_mag = product_twists.axis_vectors[orbit.first()].mag().recip();
                for axis in orbit.axes() {
                    if let Some((min_height, max_height)) = piece_data
                        .polytope
                        .height_on_axis(&product_twists.axis_vectors[axis])
                    {
                        piece_data.grip_signature[axis] = cut_distances
                            .layer_range_for_distance_range(
                                max_height * recip_mag,
                                min_height * recip_mag,
                            );
                    }
                }
            }
        }

        let axis_layers_per_orbit = axis_orbit_cut_distances
            .iter()
            .map(|d| d.layers_info())
            .collect();

        Ok(Self {
            factor_ids: vec![id],
            factor_names: vec![name],
            shape,
            colors,
            twists,
            axis_layers_per_orbit,
        })
    }

    /// Returns the direct product of two puzzles.
    ///
    /// The direct product of two puzzles `a` and `b` will have dimension
    /// `a.ndim() + b.ndim()`, with puzzle `a` occupying the lower dimensions
    /// and puzzle `b` occupying the higher dimensions.
    pub fn direct_product(&self, rhs: &Self) -> Result<Self> {
        Ok(PuzzleProduct {
            factor_ids: crate::chain_cloned(&self.factor_ids, &rhs.factor_ids),
            factor_names: crate::chain_cloned(&self.factor_names, &rhs.factor_names),
            shape: self.shape.direct_product(&rhs.shape)?,
            colors: MaybeAdHoc::AdHoc(ColorSystemDisjointUnion::disjoint_union(
                &ColorSystemDisjointUnion::from_color_system(&self.colors),
                &ColorSystemDisjointUnion::from_color_system(&rhs.colors),
            )?),
            twists: MaybeAdHoc::AdHoc(TwistSystemProduct::direct_product(
                self.twists.as_ad_hoc_or_component()?,
                rhs.twists.as_ad_hoc_or_component()?,
            )?),
            axis_layers_per_orbit: crate::chain_cloned(
                &self.axis_layers_per_orbit,
                &rhs.axis_layers_per_orbit,
            ),
        })
    }

    /// Constructs the final puzzle.
    pub fn build(
        &self,
        build_ctx: &BuildCtx,
        warn_fn: &mut impl FnMut(eyre::Report),
    ) -> Result<Arc<Puzzle>> {
        let colors = match &self.colors {
            MaybeAdHoc::Fixed(f) => Arc::clone(f),
            MaybeAdHoc::AdHoc(a) => a.build(build_ctx, warn_fn)?,
        };

        let twists = match &self.twists {
            MaybeAdHoc::Fixed(f) => Arc::clone(&f),
            MaybeAdHoc::AdHoc(a) => a.build(build_ctx, warn_fn)?,
        };

        build_ctx.set_building::<Puzzle>();

        let ndim = self.ndim();
        let piece_count = self.shape.pieces.len();

        let id = self.id();
        let name = self.name();

        let (pieces, stickers) = self.shape.build_piece_and_stickers()?;

        let (piece_types, piece_type_hierarchy, piece_type_masks) =
            self.shape.build_piece_types(warn_fn)?;

        let product_twists = twists.components.get::<TwistSystemProduct>()?;
        let symmetric_twist_system_component =
            twists.components.get::<SymmetricTwistSystemComponent>()?;

        let grip_signatures = Arc::new(self.shape.build_grip_signatures());

        let axis_layers: PerAxis<AxisLayersInfo> = self
            .axis_layers_per_orbit
            .iter()
            .zip(&product_twists.axis_orbits)
            .flat_map(|(&layers_info, axis_orbit)| std::iter::repeat_n(layers_info, axis_orbit.len))
            .collect();

        let axes_with_twists: Vec<Axis> = self
            .axis_layers_per_orbit
            .iter()
            .zip(&product_twists.axis_orbits)
            .filter(|(layers_info, orbit)| {
                layers_info.max_layer > 0
                    && symmetric_twist_system_component.axis_has_twists(orbit.first())
            })
            .flat_map(|(_, orbit)| orbit.axes())
            .collect();

        let mut mesh = self.shape.build_mesh()?;

        let mut gizmo_twists = PerGizmoFace::new();
        if ndim == 3 {
            gizmos::build_3d_gizmo(
                &mut mesh,
                &mut gizmo_twists,
                &product_twists,
                &symmetric_twist_system_component,
            )
        } else if ndim == 4 {
            gizmos::build_4d_gizmo(
                &mut mesh,
                &mut gizmo_twists,
                &product_twists,
                &symmetric_twist_system_component,
                warn_fn,
            )
        } else {
            Ok(())
        }
        .wrap_err("error building gizmos")?;

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

            axis_vectors: twists.axes.components.get()?,
            axis_layer_depths: PerAxis::new(), // TODO: is this needed?

            gizmo_twists,
        });

        let random_move = Box::new({
            let symmetric_twist_system_component = Arc::clone(&symmetric_twist_system_component);
            let axis_layers = Arc::new(axis_layers.clone());
            move |rng: &mut dyn rand::Rng| {
                let axis = *axes_with_twists.choose(rng)?;
                // TODO: avoid total layer mask when that covers all pieces
                let layers =
                    hyperpuzzle_core::util::random_layer_mask(rng, axis_layers[axis].max_layer)?;
                let family = &symmetric_twist_system_component.axes.names[axis];
                if let Some(unit_twist_order) =
                    symmetric_twist_system_component.unit_twist_order(axis)
                {
                    let order = unit_twist_order.get();
                    let mut multiplier = rng.random_range(1..order); // guaranteed nonempty
                    if multiplier * 2 > order {
                        multiplier -= order;
                    }
                    Some(Move::new(layers, family, None, multiplier))
                } else {
                    let constraints = Some(
                        symmetric_twist_system_component.random_constraints_on_axis(rng, axis)?,
                    )
                    .filter(|c| !c.constraints.is_empty());
                    Some(Move::new(layers, family, constraints, 1))
                }
            }
        });

        let mut components = ComponentList::new();
        components.insert(geom);

        Ok(Arc::new_cyclic(|this| Puzzle {
            this: Weak::clone(this),
            meta: Arc::new(CatalogMetadata {
                id,
                version: Version {
                    major: 0,
                    minor: 0,
                    patch: 1,
                },
                name: self.factor_names.iter().join(" × "),
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
            new: Box::new({
                move |ty| {
                    ProductPuzzleState {
                        ty,
                        twists: Arc::clone(&symmetric_twist_system_component),
                        piece_grip_signatures: Arc::clone(&grip_signatures),
                        piece_attitudes: PerPiece::new_with_len(piece_count),
                    }
                    .into()
                }
            }),
            random_move,
            components,
        }))
    }

    pub fn id(&self) -> CatalogId {
        crate::product_id(&self.factor_ids)
    }

    pub fn name(&self) -> String {
        crate::product_name(&self.factor_names)
    }
}

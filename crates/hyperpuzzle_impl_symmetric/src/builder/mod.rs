//! Symmetric Euclidean puzzle simulation backend and Hyperpuzzlescript API for
//! Hyperspeedcube.

use std::sync::{Arc, Weak};

use eyre::{Context, OptionExt, Result, bail};
use hypergroup::CoxeterMatrix;
use hypermath::prelude::*;
use hyperpuzzle_core::ComponentList;
use hyperpuzzle_core::catalog::BuildCtx;
use hyperpuzzle_core::prelude::*;
use hyperpuzzle_impl_nd_euclid::{NdEuclidAxisVectors, NdEuclidPuzzleGeometry};

mod colors;
mod from_space;
mod gizmos;
mod shape;
mod twists;

pub(crate) use colors::ColorSystemDisjointUnion;
use hypuz_notation::family::SequentialLowercaseName;
use itertools::Itertools;
use rand::RngExt;
use rand::seq::IndexedRandom;
use shape::{
    DisjointUnionColorName, PieceData, PieceFacetData, ProductPuzzleShape, StickerData, SurfaceData,
};
pub(crate) use twists::TwistSystemProduct;

use crate::spec::SimpleOrbitSpec;
use crate::{CutDistances, ProductPuzzleState, SymmetricTwistSystemComponent};

#[derive(Debug)]
pub struct PuzzleProduct {
    id: CatalogId,
    factors: Vec<PuzzleProductFactor>,
    shape: ProductPuzzleShape,
    axis_layers_per_orbit: Vec<AxisLayersInfo>,
}

impl CatalogObject for PuzzleProduct {
    fn catalog_type_name() -> &'static str {
        "factor puzzle"
    }

    fn id(&self) -> &CatalogId {
        &self.id
    }
}

impl PuzzleProduct {
    /// Returns the number of the dimensions of the puzzle.
    pub fn ndim(&self) -> u8 {
        self.shape.ndim()
    }

    /// Constructs the empty puzzle, which is the identity of the direct
    /// product.
    pub fn direct_product_identity() -> Self {
        PuzzleProduct {
            id: crate::product_id([].into_iter()),
            factors: vec![],
            shape: ProductPuzzleShape::direct_product_identity(),
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
        facet_orbits: &[SimpleOrbitSpec],
        colors_id: CatalogId,
        twists: &TwistSystemProduct,
        axis_orbit_cut_distances: &[CutDistances],
        warn_fn: &mut impl FnMut(eyre::Report),
    ) -> Result<Self> {
        let group = coxeter_matrix.isometry_group()?;

        let mut shape_builder = from_space::PuzzleShapeFactorBuilder::new(coxeter_matrix, group)?;

        // Carve facets
        for orbit in facet_orbits {
            for facet in &orbit.orbit_members {
                let plane = Hyperplane::from_pole(&facet.vector).ok_or_eyre("bad hyperplane")?;
                let color = DisjointUnionColorName {
                    prefix: SequentialLowercaseName(0),
                    name: facet.name.clone(),
                };
                shape_builder.carve(plane, color)?;
            }
            shape_builder.color_orbits.push(Orbit {
                elements: Arc::new(
                    orbit
                        .orbit_members
                        .iter()
                        .map(|f| {
                            Some(DisjointUnionColorName {
                                prefix: SequentialLowercaseName(0),
                                name: f.name.clone(),
                            })
                        })
                        .collect(),
                ),
                generator_sequences: Arc::new(
                    orbit
                        .orbit_members
                        .iter()
                        .map(|f| f.abbr_gen_seq.clone())
                        .collect(),
                ),
            });
        }
        shape_builder.set_surface_centroids_from_stickers_of_single_piece()?;

        // Slice axes
        for (orbit, cut_distances) in std::iter::zip(twists.axis_orbits(), axis_orbit_cut_distances)
        {
            for axis in orbit.axes() {
                for &cut_distance in &cut_distances.0 {
                    let plane = Hyperplane::new(&twists.axis_vectors[axis], cut_distance)
                        .ok_or_eyre("bad axis vector")?;
                    shape_builder.slice(plane)?;
                }
            }
        }

        let mut shape = shape_builder.into_product_puzzle_shape()?;

        // Add grip signatures
        for (_, piece_data) in &mut shape.pieces {
            piece_data.grip_signature = PerAxis::new_with_len(twists.len());
            for (orbit, cut_distances) in
                std::iter::zip(twists.axis_orbits(), axis_orbit_cut_distances)
            {
                let recip_mag = twists.axis_vectors[orbit.first()].mag().recip();
                for axis in orbit.axes() {
                    if let Some((min_height, max_height)) = piece_data
                        .polytope
                        .height_on_axis(&twists.axis_vectors[axis])
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

        if shape.ndim() != twists.ndim() {
            bail!(
                "shape has ndim={} but twist system has ndim={}",
                shape.ndim(),
                twists.ndim(),
            );
        }

        Ok(Self {
            id: crate::product_id([&id].into_iter()),
            factors: vec![PuzzleProductFactor {
                id,
                name,
                colors_id,
                twists_id: twists.id.clone(),
            }],
            shape,
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
            id: crate::product_id(
                std::iter::chain(&self.factors, &rhs.factors)
                    .map(|f| &f.id)
                    .collect_vec()
                    .into_iter(),
            ),
            factors: crate::chain_cloned(&self.factors, &rhs.factors),
            shape: self.shape.direct_product(&rhs.shape)?,
            axis_layers_per_orbit: crate::chain_cloned(
                &self.axis_layers_per_orbit,
                &rhs.axis_layers_per_orbit,
            ),
        })
    }

    pub fn colors_id(&self) -> CatalogId {
        crate::disjoint_union_id(self.factors.iter().map(|f| &f.colors_id))
    }
    pub fn twists_id(&self) -> CatalogId {
        crate::product_id(self.factors.iter().map(|f| &f.twists_id))
    }

    /// Constructs the final puzzle.
    pub fn build(
        &self,
        build_ctx: &BuildCtx,
        colors: Arc<ColorSystem>,
        twists: Arc<TwistSystem>,
        warn_fn: &mut impl FnMut(eyre::Report),
    ) -> Result<Arc<Puzzle>> {
        let ndim = self.ndim();
        let piece_count = self.shape.pieces.len();

        let (pieces, stickers) = self.shape.build_piece_and_stickers(&colors)?;

        let (piece_types, piece_type_hierarchy, piece_type_masks) =
            self.shape.build_piece_types(warn_fn)?;

        let axis_vectors = twists.axes.components.get::<NdEuclidAxisVectors>()?;
        let symmetric_twist_system_component =
            twists.components.get::<SymmetricTwistSystemComponent>()?;

        let grip_signatures = Arc::new(self.shape.build_grip_signatures());

        let axis_layers: PerAxis<AxisLayersInfo> = self
            .axis_layers_per_orbit
            .iter()
            .zip(&*symmetric_twist_system_component.axis_orbits)
            .flat_map(|(&layers_info, orbit)| std::iter::repeat_n(layers_info, orbit.len))
            .collect();

        let axes_with_twists: Vec<Axis> = self
            .axis_layers_per_orbit
            .iter()
            .zip(&*symmetric_twist_system_component.axis_orbits)
            .filter(|(layers_info, orbit)| {
                layers_info.max_layer > 0
                    && symmetric_twist_system_component.axis_has_twists(orbit.first)
            })
            .flat_map(|(_, orbit)| orbit.axes())
            .collect();

        let mut mesh = self.shape.build_mesh()?;

        let mut gizmo_twists = PerGizmoFace::new();
        if !symmetric_twist_system_component.axes.is_empty() {
            if ndim == 3 {
                gizmos::build_3d_gizmo(
                    &mut mesh,
                    &mut gizmo_twists,
                    &axis_vectors.vectors_by_id,
                    &symmetric_twist_system_component,
                )
                .wrap_err("error building 3D gizmos")?;
            } else if ndim == 4 {
                gizmos::build_4d_gizmo(
                    &mut mesh,
                    &mut gizmo_twists,
                    &axis_vectors.vectors_by_id,
                    &symmetric_twist_system_component,
                    warn_fn,
                )
                .wrap_err("error building 4D gizmos")?;
            }
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

        Ok(Arc::new_cyclic(move |this| Puzzle {
            this: Weak::clone(this),
            meta: Arc::new(PuzzleListEntry {
                id: self.id.clone(),
                // TODO: somehow capture version info for factor puzzles
                version: Some(Version {
                    major: 1,
                    minor: 0,
                    patch: 0,
                }),
                name: self.name(),
                aliases: vec![],
                tags: TagSet::new(), // TODO: tags for products
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

    pub fn id(&self) -> &CatalogId {
        &self.id
    }

    pub fn name(&self) -> String {
        crate::product_name(self.factors.iter().map(|f| &f.name))
    }
}

#[derive(Debug, Clone)]
struct PuzzleProductFactor {
    id: CatalogId,
    name: String,
    colors_id: CatalogId,
    twists_id: CatalogId,
}

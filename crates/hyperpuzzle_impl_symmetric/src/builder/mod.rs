//! Symmetric Euclidean puzzle simulation backend and Hyperpuzzlescript API for
//! Hyperspeedcube.

use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Weak};

use eyre::{Context, OptionExt, Result, bail, eyre};
use hypergroup::AbbrGenSeq;
use hypermath::prelude::*;
use hyperpuzzle_core::ComponentList;
use hyperpuzzle_core::catalog::BuildCtx;
use hyperpuzzle_core::prelude::*;
use hyperpuzzle_impl_nd_euclid::{NdEuclidAxisVectors, NdEuclidPuzzleGeometry};

mod colors;
mod from_space;
mod gizmos;
mod names;
mod shape;
mod twists;

pub(crate) use colors::ColorSystemDisjointUnion;
use hypuz_notation::family::SequentialLowercaseName;
use itertools::Itertools;
use names::*;
use rand::RngExt;
use rand::seq::IndexedRandom;
use shape::{
    DisjointUnionColorName, PieceData, PieceFacetData, ProductPuzzleShape, StickerData, SurfaceData,
};
pub(crate) use twists::TwistSystemProduct;

use crate::{FactorPuzzleSpec, NamedPoint, ProductPuzzleState, SymmetricTwistSystemComponent};

#[derive(Debug)]
pub struct PuzzleProduct {
    id: CatalogId,
    factors: Vec<PuzzleProductFactor>,
    shape: ProductPuzzleShape,
    axis_layers_per_orbit: Vec<AxisLayersInfo>, // TODO: may be redundant with layer ranges
    axis_layer_ranges_per_orbit: Vec<PerLayer<[Float; 2]>>,
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
            axis_layer_ranges_per_orbit: vec![],
        }
    }

    /// Constructs a factor puzzle builder.
    ///
    /// Note that `axis_orbit_cut_distances` must be sorted from outermost
    /// (greatest) to innermost (least).
    pub fn new_factor(
        build_ctx: &BuildCtx,
        spec: &FactorPuzzleSpec,
        warn_fn: &mut impl FnMut(eyre::Report),
    ) -> Result<Self> {
        build_ctx.push_task("constructing puzzle isometry group");
        let group = spec.coxeter_matrix.isometry_group()?;
        build_ctx.pop_task();

        build_ctx.push_task("initializing shape builder");
        let mut shape_builder = from_space::PuzzleShapeFactorBuilder::new(
            spec.coxeter_matrix.clone(),
            group.clone(),
            spec.primordial_cube_radius(),
        )?;
        build_ctx.pop_task();

        build_ctx.push_task("naming named points");
        let (named_point_vectors, named_point_unit_vectors, _named_point_orbits, named_point_names) =
            FactorNamedPointBasedNames::<Axis>::from_spec(&group, &spec.named_point_orbits)?;
        build_ctx.pop_task();

        build_ctx.push_task("constructing named point group action");
        let named_point_points = named_point_vectors.map_ref(|_, v| Point(v.clone()));
        let named_point_action = group.action_on_points(&named_point_points)?;
        build_ctx.pop_task();

        // Carve facets
        for orbit in &spec.facet_orbits {
            build_ctx.push_task(format!("carving facets with prefix {:?}", orbit.prefix));

            build_ctx.push_task("expanding orbit");
            let orbit_members =
                orbit.expand_and_name(&group, &named_point_unit_vectors, &named_point_action)?;
            build_ctx.pop_task();
            let mut orbit_elements = vec![];
            let mut orbit_generator_sequences = vec![];
            let mut gen_seq_to_name = HashMap::new();
            for (elem, facet_pole_vector, named_point_sets) in orbit_members {
                let plane =
                    Hyperplane::from_pole(facet_pole_vector).ok_or_eyre("bad hyperplane")?;

                // Color names are just 1-to-1 with strings because the
                // backwards-compat concerns aren't as serious as for axes.
                let mut facet_name = orbit.prefix.clone();
                for p in named_point_sets.into_iter().flatten() {
                    facet_name += &**named_point_names.named_point_names().get_name(p)?;
                }
                let color = DisjointUnionColorName {
                    prefix: SequentialLowercaseName(0),
                    name: facet_name,
                };

                orbit_elements.push(Some(color.clone()));
                // This could be done more easily by using
                // `orbit_geometric_with_gen_seq()` but oh well.
                let factorization = group.factorization(elem).collect_vec();
                gen_seq_to_name.insert(factorization.clone(), gen_seq_to_name.len());
                let abbr_gen_seq = if let Some(tail) = factorization.get(1..)
                    && let Some(&end) = gen_seq_to_name.get(tail)
                {
                    AbbrGenSeq::new([factorization[0]], Some(end))
                } else {
                    AbbrGenSeq::new(factorization, None)
                };
                orbit_generator_sequences.push(abbr_gen_seq);

                shape_builder.carve(plane, color)?;
            }
            shape_builder.color_orbits.push(Orbit {
                elements: Arc::new(orbit_elements),
                generator_sequences: Arc::new(orbit_generator_sequences),
            });

            build_ctx.pop_task();
        }
        build_ctx.push_task("computing surface centroids");
        shape_builder.set_surface_centroids_from_stickers_of_single_piece()?;
        build_ctx.pop_task();

        // Slice axes
        if let Some(twists) = &spec.twists {
            build_ctx.push_task("slicing axes");
            for (orbit, cut_distances) in
                std::iter::zip(twists.axis_orbits(), &spec.axis_orbit_cut_distances)
            {
                for axis in orbit.axes() {
                    for &cut_distance in cut_distances.distances() {
                        let plane = Hyperplane::new(&twists.axis_vectors[axis], cut_distance)
                            .ok_or_eyre("bad axis vector")?;
                        shape_builder.slice(plane)?;
                    }
                }
            }
            build_ctx.pop_task();
        } else if !spec.axis_orbit_cut_distances.is_empty() {
            warn_fn(eyre!("ignoring cut distances for empty twist system"));
        }

        build_ctx.push_task("building shape");
        let mut shape = shape_builder.into_product_puzzle_shape()?;
        build_ctx.pop_task();

        // Add grip signatures
        if let Some(twists) = &spec.twists {
            build_ctx.push_task("computing grip signatures");
            for (_, piece_data) in &mut shape.pieces {
                piece_data.grip_signature = PerAxis::new_with_len(twists.len());
                for (orbit, cut_distances) in
                    std::iter::zip(twists.axis_orbits(), &spec.axis_orbit_cut_distances)
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
            build_ctx.pop_task();
        }

        let axis_layers_per_orbit = spec
            .axis_orbit_cut_distances
            .iter()
            .map(|d| d.layers_info())
            .collect();

        let axis_layer_ranges_per_orbit = spec
            .axis_orbit_cut_distances
            .iter()
            .map(|d| d.distances().array_windows().copied().collect())
            .collect();

        if let Some(twists) = &spec.twists
            && shape.ndim() != twists.ndim()
        {
            bail!(
                "shape has ndim={} but twist system has ndim={}",
                shape.ndim(),
                twists.ndim(),
            );
        }

        Ok(Self {
            id: crate::product_id([&spec.id].into_iter()),
            factors: vec![PuzzleProductFactor {
                id: spec.id.clone(),
                name: spec.name.clone(),
                colors_id: spec.colors_id.clone(),
                twists_id: spec.twists.as_ref().map(|t| t.id.clone()),
            }],
            shape,
            axis_layers_per_orbit,
            axis_layer_ranges_per_orbit,
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
            axis_layer_ranges_per_orbit: crate::chain_cloned(
                &self.axis_layer_ranges_per_orbit,
                &rhs.axis_layer_ranges_per_orbit,
            ),
        })
    }

    pub fn colors_id(&self) -> Option<CatalogId> {
        Some(crate::disjoint_union_id(
            self.factors
                .iter()
                .map(|f| f.colors_id.as_ref())
                .collect::<Option<Vec<_>>>()?
                .into_iter(),
        ))
    }
    pub fn twists_id(&self) -> Option<CatalogId> {
        Some(crate::product_id(
            self.factors
                .iter()
                .map(|f| f.twists_id.as_ref())
                .collect::<Option<Vec<_>>>()?
                .into_iter(),
        ))
    }

    /// Constructs the final puzzle.
    pub fn build(
        &self,
        _build_ctx: &BuildCtx,
        meta: Arc<PuzzleListEntry>,
        colors: Arc<ColorSystem>,
        twists: Arc<TwistSystem>,
        warn_fn: &mut impl FnMut(eyre::Report),
    ) -> Result<Arc<Puzzle>> {
        let ndim = self.ndim();

        // TODO: actually measure perf on, e.g., FT 600-Cell
        let mut shape = self.shape.clone();
        if ndim > 3 {
            shape.remove_internals();
        }

        let (pieces, stickers) = shape.build_piece_and_stickers(&colors)?;

        let (piece_types, piece_type_hierarchy, piece_type_masks) =
            shape.build_piece_types(warn_fn)?;

        let axis_vectors = twists.axes.components.get::<NdEuclidAxisVectors>()?;
        let symmetric_twist_system_component =
            twists.components.get::<SymmetricTwistSystemComponent>()?;

        let grip_signatures = Arc::new(shape.build_grip_signatures());

        let axis_layers: Arc<PerAxis<AxisLayersInfo>> = Arc::new(
            self.axis_layers_per_orbit
                .iter()
                .zip(&*symmetric_twist_system_component.axis_orbits)
                .flat_map(|(&layers_info, orbit)| std::iter::repeat_n(layers_info, orbit.len))
                .collect(),
        );
        // For each axis, compute whether its layers combine to contain every
        // piece.
        let mut does_axis_contain_every_piece = axis_layers.map_ref(|_, _| true);
        for (_piece, piece_grip_signatures) in &*grip_signatures {
            // TODO: instead of basing this on grip signatures, look for ±inf cut depths
            for (axis, layer_range) in piece_grip_signatures {
                if layer_range.is_none() {
                    does_axis_contain_every_piece[axis] = false;
                }
            }
        }

        let axis_layer_ranges = Arc::new(
            self.axis_layer_ranges_per_orbit
                .iter()
                .zip(&*symmetric_twist_system_component.axis_orbits)
                .flat_map(|(layer_ranges, orbit)| {
                    std::iter::repeat_n(layer_ranges.clone(), orbit.len)
                })
                .collect(),
        );

        let axes_with_nontrivial_twists: Vec<Axis> = self
            .axis_layers_per_orbit
            .iter()
            .zip(&*symmetric_twist_system_component.axis_orbits)
            .filter(|(layers_info, orbit)| {
                layers_info.max_layer > 0
                    && symmetric_twist_system_component.axis_has_twists(orbit.first)
            })
            .flat_map(|(layers_info, orbit)| {
                orbit.axes().filter(|&axis| {
                    !does_axis_contain_every_piece[axis] || layers_info.max_layer > 1
                })
            })
            .collect();

        let mut mesh = shape.build_mesh()?;

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
                    &mut *warn_fn,
                )
                .wrap_err("error building 4D gizmos")?;
            }
        }
        let gizmo_axes = Arc::new(gizmo_twists.try_map_ref(|_, mv| {
            (twists.axis_from_family)(&mv.transform.family)
                .ok_or_else(|| eyre!("missing axis for gizmo twist {mv:?}"))
        })?);
        // `&_` is required to work around https://github.com/rust-lang/rust/issues/58052
        let axis_names = Arc::clone(&twists.axes.names);
        let symmetric_twist_system_component_ref = Arc::clone(&symmetric_twist_system_component);
        let get_gizmo_twist = Box::new(
            move |gizmo_face: GizmoFace,
                  layers: Option<LayerMask>,
                  direction: RotDir,
                  state: &dyn PuzzleState| {
                // TODO: store twist family string and/or axis ID, not Move
                let mut twist = gizmo_twists[gizmo_face].clone();

                let gizmo_string = twist.transform.family.to_string();

                let layer_mask = layers.unwrap_or(LayerMask::from_layer(Layer::MIN));
                twist.layers = layer_mask.clone().into();

                // Handle jumbling
                let dir_sign = direction.to_sign(RotDir::Cw);
                if let Some(axis) = axis_names.lookup(&twist.transform.family)
                    && let (_, orbit_index) =
                        symmetric_twist_system_component_ref.axis_undeorbiters[axis]
                    && let Some(jumble_data) =
                        &symmetric_twist_system_component_ref.axis_orbits[orbit_index].jumble_data
                    && let Some(state) =
                        (state as &dyn Any).downcast_ref::<crate::ProductPuzzleState>()
                    && let Some(jumble_states) = &state.axis_jumble_states
                    && let Some(first_layer) = layer_mask.iter().next()
                    && let old_stop = jumble_states[axis][first_layer]
                    && let new_stop = jumble_data.adjacent_stop(old_stop, dir_sign)
                    && let Ok(jumble_transforms) =
                        jumble_data.notation_from_stop_to_stop(old_stop, new_stop, Some(dir_sign))
                {
                    // TODO: consider all layers, and select the next jumble stop that is ok for all of them
                    return Some((
                        gizmo_string,
                        jumble_transforms
                            .into_iter()
                            .map(|j| {
                                j.on_axis_with_layers(
                                    twist.layers.clone(),
                                    &twist.transform.family,
                                    "_", // TODO: correct number of underscores (may be 0)
                                )
                            })
                            .collect(),
                    ));
                }

                if direction == RotDir::Ccw
                    && let Ok(inv_mult) = twist.multiplier.inv()
                {
                    twist.multiplier = inv_mult;
                }
                Some((gizmo_string, vec![twist]))
            },
        );

        let (planes, sticker_planes) = shape.build_sticker_planes();

        let geom = Arc::new(NdEuclidPuzzleGeometry {
            vertex_coordinates: vec![],
            piece_vertex_sets: PerPiece::new_with_len(shape.pieces.len()),
            piece_centroids: shape.build_piece_centroids(),

            planes,
            sticker_planes,

            mesh,

            axis_vectors: twists.axes.components.get()?,
            axis_layer_depths: PerAxis::new(), // TODO: is this needed?

            gizmo_axes,
            get_gizmo_twist,
        });

        let random_move = Box::new({
            let symmetric_twist_system_component = Arc::clone(&symmetric_twist_system_component);
            let axis_layers = Arc::clone(&axis_layers);
            move |rng: &mut dyn rand::Rng| {
                let axis = *axes_with_nontrivial_twists.choose(rng)?;
                let all_layers = LayerMask::all(axis_layers[axis].max_layer);
                let layers =
                    hyperpuzzle_core::util::random_layer_masks(rng, axis_layers[axis].max_layer)
                        .take(1000) // abort if failed too many times
                        .find(|layer_mask| {
                            !layer_mask.is_empty()
                                && (!does_axis_contain_every_piece[axis]
                                    || *layer_mask != all_layers)
                        })?;
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

        // Check that stable puzzle doesn't depend on unstable colors/twists
        if self.id.base.version.unwrap_or(0) > 0 {
            if colors.id.base.version.unwrap_or(0) == 0 {
                warn_fn(eyre!(
                    "stable puzzle {} depends on unstable color system {}",
                    self.id,
                    colors.id,
                ));
            }
            if twists.id.base.version.unwrap_or(0) == 0 {
                warn_fn(eyre!(
                    "stable puzzle {} depends on unstable twist system {}",
                    self.id,
                    twists.id,
                ));
            }
        }

        let piece_points = Arc::new(shape.pieces.map_ref(|_, piece_data| {
            piece_data
                .polytope
                .verts
                .iter()
                .map(|(_, xs)| Point::from_iter(xs.iter().copied()))
                .collect()
        }));

        let any_jumbling = symmetric_twist_system_component
            .axis_orbits
            .iter()
            .any(|orbit| orbit.jumble_data.is_some());

        Ok(Arc::new_cyclic(move |this| Puzzle {
            this: Weak::clone(this),
            meta,
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
            axis_layers: Arc::clone(&axis_layers),
            twists,
            new: Box::new(move |ty| {
                ProductPuzzleState {
                    ty,
                    twists: Arc::clone(&symmetric_twist_system_component),
                    piece_grip_signatures: Arc::clone(&grip_signatures),
                    piece_points: Arc::clone(&piece_points),
                    axis_layer_ranges: Arc::clone(&axis_layer_ranges),
                    axis_vectors: Arc::clone(&axis_vectors),
                    axis_jumble_states: any_jumbling.then(|| {
                        axis_layers.map_ref(|_, layers_info| {
                            PerLayer::new_with_len(layers_info.max_layer as usize)
                        })
                    }),
                    piece_attitudes: PerPiece::new_with_len(shape.pieces.len()),
                }
                .into()
            }),
            random_move,
            components,
        }))
    }

    pub fn build_ad_hoc_color_system(&self) -> Result<Arc<ColorSystem>> {
        self.shape.build_ad_hoc_color_system(self.id.clone())
    }

    pub fn name(&self) -> String {
        crate::product_name(self.factors.iter().map(|f| &f.name))
    }
}

#[derive(Debug, Clone)]
struct PuzzleProductFactor {
    id: CatalogId,
    name: String,
    /// Color system ID, or `None` to use an ad-hoc color system.
    colors_id: Option<CatalogId>,
    /// Twist system ID, or `None` to use no twists.
    twists_id: Option<CatalogId>,
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

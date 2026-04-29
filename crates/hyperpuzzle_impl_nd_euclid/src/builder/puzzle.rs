use std::collections::HashMap;
use std::sync::{Arc, Weak};

use eyre::{OptionExt, Result, ensure};
use hypermath::prelude::*;
use hyperpuzzle_core::catalog::BuildCtx;
use hyperpuzzle_core::prelude::*;
use hyperpuzzle_core::util::MaybeAdHoc;
use hyperpuzzle_core::{ComponentList, Move};
use hypershape::prelude::*;
use itertools::Itertools;
use parking_lot::Mutex;
use rand::seq::IndexedRandom;
use rand::{Rng, RngExt};
use smallvec::{SmallVec, smallvec};
use tinyset::Set64;

use super::shape::ShapeBuildOutput;
use super::{AdHocTwistSystemBuilder, AxisLayersBuilder, ShapeBuilder, TwistSystemBuilder};
use crate::components::NdEuclidTwistsList;
use crate::{NamedTwistsList, prelude::*};

/// [`Puzzle`] under construction.
#[derive(Debug)]
pub struct PuzzleBuilder {
    /// Puzzle metadata.
    pub meta: Arc<CatalogMetadata>,

    /// Number of dimensions of the underlying space the puzzle is built in.
    pub ndim: u8,
    /// Shape of the puzzle.
    pub shape: Arc<Mutex<ShapeBuilder>>,
    /// Twist system of the puzzle.
    pub twists: TwistSystemBuilder,

    /// Layer data for each layer on the axis, in order from outermost to
    /// innermost.
    ///
    /// Axes may be missing from this! Always ensure it is long enough before
    /// mutating.
    pub axis_layers: PerAxis<AxisLayersBuilder>,

    /// Number of moves for a full scramble.
    pub full_scramble_length: u32,
}
impl PuzzleBuilder {
    /// Constructs a new puzzle builder with a primordial cube.
    pub fn new(meta: Arc<CatalogMetadata>, ndim: u8) -> Result<Self> {
        let (min, max) = (Space::MIN_NDIM, Space::MAX_NDIM);
        ensure!(ndim >= min, "ndim={ndim} is below min value of {min}");
        ensure!(ndim <= max, "ndim={ndim} exceeds max value of {max}");
        let shape = ShapeBuilder::new_with_primordial_cube(&meta.id, ndim)?;
        let twists = AdHocTwistSystemBuilder::new(meta.id.clone(), None, ndim);
        Ok(Self {
            meta,

            ndim,
            shape: Arc::new(Mutex::new(shape)),
            twists: TwistSystemBuilder(MaybeAdHoc::AdHoc(Arc::new(Mutex::new(twists)))),

            axis_layers: PerAxis::new(),

            full_scramble_length: hyperpuzzle_core::FULL_SCRAMBLE_LENGTH,
        })
    }

    /// Returns the number of dimensions of the underlying space the puzzle is
    /// built in.
    pub fn ndim(&self) -> u8 {
        self.ndim
    }

    /// Returns a mutable reference to the axis layers. All layers are
    /// guaranteed to exist.
    pub fn axis_layers(&self, axis: Axis) -> &AxisLayersBuilder {
        self.axis_layers
            .get(axis)
            .unwrap_or(const { &AxisLayersBuilder::new() })
    }
    /// Returns a union-of-intersections of bounded regions for the given layer
    /// mask.
    pub fn plane_bounded_regions(
        &self,
        axis: Axis,
        axis_vector: &Vector,
        layer_mask: LayerMask,
    ) -> Result<Vec<SmallVec<[Hyperplane; 2]>>> {
        // TODO: optimize by removing overlapping planes
        layer_mask
            .iter()
            .map(|layer| self.boundary_of_layer(axis, axis_vector, layer))
            .collect()
    }
    /// Returns the hyperplanes bounding a layer.
    fn boundary_of_layer(
        &self,
        axis: Axis,
        axis_vector: &Vector,
        layer: Layer,
    ) -> Result<SmallVec<[Hyperplane; 2]>> {
        let layers = self.axis_layers(axis);

        let l = layers.0.get(layer)?;
        let mut ret = smallvec![];
        if l.top.is_finite() {
            ret.push(Hyperplane::new(axis_vector, l.top).ok_or_eyre("bad axis vector")?);
        }
        if l.bottom.is_finite() {
            ret.push(
                Hyperplane::new(axis_vector, l.bottom)
                    .ok_or_eyre("bad axis vector")?
                    .flip(),
            );
        }
        Ok(ret)
    }

    /// Performs the final steps of building a puzzle, generating the mesh and
    /// assigning IDs to pieces, stickers, etc.
    pub fn build(
        &self,
        build_ctx: Option<&BuildCtx>,
        warn_fn: &mut impl FnMut(eyre::Error),
    ) -> Result<Arc<Puzzle>> {
        let mut shape_builder = self.shape.lock();
        let colors_builder = &shape_builder.colors;

        // Build color system.
        let colors = match &colors_builder.0 {
            MaybeAdHoc::Fixed(f) => Arc::clone(f),
            MaybeAdHoc::AdHoc(a) => a.build(build_ctx, warn_fn)?,
        };

        // Build twist system.
        let twists = match &self.twists.0 {
            MaybeAdHoc::Fixed(f) => Arc::clone(f),
            MaybeAdHoc::AdHoc(a) => a.lock().build(build_ctx, warn_fn)?,
        };

        if let Some(build_ctx) = build_ctx {
            build_ctx.set_building::<Puzzle>();
        }

        // Build shape.
        let ShapeBuildOutput {
            mut mesh,
            pieces,
            piece_polytopes,
            stickers,
            sticker_planes,

            piece_types,
            piece_type_hierarchy,
            piece_type_masks,
        } = shape_builder.build(warn_fn)?;

        let space = &mut shape_builder.space;
        let ndim = space.ndim();

        // Build twist gizmos.
        let gizmo_twists = super::gizmos::build_twist_gizmos(space, &mut mesh, &twists, warn_fn)?;

        // Build vertex sets.
        let mut vertex_count = 0;
        let mut vertex_coordinates = vec![];
        let mut vertex_id_map = HashMap::new();
        let piece_vertex_sets: TiVec<Piece, Set64<usize>> =
            piece_polytopes.map(|_piece, polytope_id| {
                space
                    .get(polytope_id)
                    .vertex_set()
                    .map(|v| {
                        *vertex_id_map.entry(v.id()).or_insert_with(|| {
                            vertex_coordinates.extend(v.pos().as_vector().iter_ndim(ndim));
                            let i = vertex_count;
                            vertex_count += 1;
                            i
                        })
                    })
                    .collect()
            });

        // Build piece center points.
        let piece_centroids = piece_vertex_sets.map_ref(|_, point_set| {
            (0..ndim as usize)
                .map(|j| {
                    point_set
                        .iter()
                        .map(|v| vertex_coordinates[v * ndim as usize + j])
                        .sum()
                })
                .collect()
        });

        // Build hyperplanes.
        let mut planes = vec![];
        let mut plane_id_map = ApproxHashMap::new(APPROX);
        let sticker_planes = sticker_planes.map(|_sticker, plane| {
            *plane_id_map.entry(plane.clone()).or_insert_with(|| {
                let i = planes.len();
                planes.push(plane);
                i
            })
        });

        // Build layers.
        let mut axis_layers = self.axis_layers.clone();
        axis_layers.resize(twists.axes.len())?;
        let axis_layer_depths = axis_layers.try_map_ref(|_, layers| layers.build())?;

        let axis_layers = axis_layer_depths.map_ref(|_, depths| AxisLayersInfo {
            max_layer: depths.len() as u16,
            allow_negatives: false, // TODO: configurable negative layers
        });

        let geom = Arc::new(NdEuclidPuzzleGeometry {
            vertex_coordinates,
            piece_vertex_sets,
            piece_centroids,

            planes,
            sticker_planes,

            mesh,

            axis_vectors: Arc::clone(twists.axes.components.get()?),
            axis_layer_depths,

            gizmo_twists,
        });

        let twists_list = Arc::clone(twists.components.get::<NdEuclidTwistsList>()?);
        let twist_names = Arc::clone(twists.components.get::<NamedTwistsList>()?);

        let mut scramble_twists = twists_list
            .iter()
            .filter(|&twist| {
                twists_list.scramble_max_multipliers[twist].is_some()
                    && axis_layers[twists_list.twist_axes[twist]].max_layer > 0
            })
            .collect_vec();
        scramble_twists.sort_by_cached_key(|&twist| match twist_names.names.get(twist) {
            Ok(name) => &name.canonical,
            Err(_) => "",
        });
        let can_scramble = !scramble_twists.is_empty();

        let random_move = Box::new({
            let axis_layers_info = axis_layers.clone();
            move |rng: &mut dyn Rng| {
                let random_twist = *scramble_twists.choose(rng)?;
                let axis = twists_list.twist_axes[random_twist];

                let layer_count = axis_layers_info[axis].max_layer;
                let random_layer_mask =
                    hyperpuzzle_core::util::random_layer_mask(rng, layer_count)?;

                let max_multiplier =
                    twists_list.scramble_max_multipliers[random_twist].unwrap_or_default();
                let random_multiplier = rng.random_range(1..=max_multiplier.0);

                Some(Move {
                    layers: random_layer_mask.into(),
                    transform: notation::Transform::new(&twist_names.names[random_twist], None),
                    multiplier: Multiplier(random_multiplier),
                })
            }
        });

        let mut components = ComponentList::new();
        components.insert(Arc::clone(&geom));

        Ok(Arc::new_cyclic(|this| Puzzle {
            this: Weak::clone(this),
            meta: self.meta.clone(),

            view_prefs_set: Some(PuzzleViewPreferencesSet::Perspective(
                PerspectiveDim::from_ndim(ndim),
            )),

            pieces,
            stickers,
            piece_types,
            piece_type_hierarchy,
            piece_type_masks,

            colors,

            can_scramble,
            full_scramble_length: self.full_scramble_length,

            axis_layers,
            twists,

            new: Box::new(move |this| NdEuclidPuzzleState::new(this, Arc::clone(&geom)).into()),

            random_move,

            components,
        }))
    }
}

/// Piece of a puzzle during puzzle construction.
#[derive(Debug, Clone)]
pub struct PieceBuilder {
    /// Polytope of the piece.
    pub polytope: PolytopeId,
    /// If the piece is defunct because it was cut, these are the pieces it was
    /// cut up into.
    pub cut_result: PieceSet,
    /// Colored stickers of the piece.
    pub stickers: VecMap<FacetId, Color>,
    /// Type of piece, if assigned.
    pub piece_type: Option<PieceType>,

    /// Cached arbitrary point inside the polytope.
    cached_interior_point: Option<Point>,
}
impl PieceBuilder {
    pub(super) fn new(polytope: Polytope<'_>, stickers: VecMap<FacetId, Color>) -> Self {
        Self {
            polytope: polytope.id(),
            cut_result: PieceSet::new(),
            stickers,
            piece_type: None,

            cached_interior_point: None,
        }
    }
    /// Returns the color of a facet, or `Color::INTERNAL` if there is no
    /// color assigned.
    pub fn sticker_color(&self, sticker_id: FacetId) -> Color {
        *self.stickers.get(&sticker_id).unwrap_or(&Color::INTERNAL)
    }

    pub(super) fn interior_point(&mut self, space: &Space) -> &Point {
        // Average the vertices to get a point that is inside the polytope. For
        // polytopes with many vertices, this could perhaps be improved by using
        // blades.
        self.cached_interior_point.get_or_insert_with(|| {
            let mut count = 0;
            let mut sum = vector![];
            for v in space.get(self.polytope).vertex_set() {
                count += 1;
                sum += v.pos().into_vector();
            }
            Point(sum / count as _)
        })
    }
}

/// Piece type of a puzzle during puzzle construction.
#[derive(Debug, Clone)]
pub struct PieceTypeBuilder {
    /// Name for the piece type. (e.g., `center/oblique_1_2/left`)
    pub name: String,
}

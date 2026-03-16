use std::collections::HashMap;
use std::sync::{Arc, Weak};

use eyre::{OptionExt, Result, ensure};
use hypermath::prelude::*;
use hyperpuzzle_core::Move;
use hyperpuzzle_core::catalog::{BuildCtx, BuildTask};
use hyperpuzzle_core::prelude::*;
use hypershape::prelude::*;
use hypuz_notation::{AxisLayersInfo, LayerRange};
use itertools::Itertools;
use parking_lot::Mutex;
use rand::Rng;
use rand::seq::IndexedRandom;
use smallvec::{SmallVec, smallvec};
use tinyset::Set64;

use super::shape::ShapeBuildOutput;
use super::{AxisLayersBuilder, ShapeBuilder, TwistSystemBuilder};
use crate::NdEuclidTwistSystemEngineData;
use crate::prelude::*;

/// Puzzle being constructed.
#[derive(Debug)]
pub struct PuzzleBuilder {
    /// Puzzle metadata.
    pub meta: Arc<PuzzleListMetadata>,

    /// Space in which the puzzle is constructed.
    pub space: Arc<Space>,
    /// Shape of the puzzle.
    pub shape: Arc<Mutex<ShapeBuilder>>,
    /// Twist system of the puzzle.
    pub twists: Arc<Mutex<TwistSystemBuilder>>,

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
    pub fn new(meta: Arc<PuzzleListMetadata>, ndim: u8) -> Result<Self> {
        let (min, max) = (Space::MIN_NDIM, Space::MAX_NDIM);
        ensure!(ndim >= min, "ndim={ndim} is below min value of {min}");
        ensure!(ndim <= max, "ndim={ndim} exceeds max value of {max}");
        let space = Space::new(ndim);
        let shape = ShapeBuilder::new_with_primordial_cube(&meta.id, Arc::clone(&space))?;
        let twists = TwistSystemBuilder::new_ad_hoc(&meta.id, ndim);
        Ok(Self {
            meta,

            space,
            shape: Arc::new(Mutex::new(shape)),
            twists: Arc::new(Mutex::new(twists)),

            axis_layers: PerAxis::new(),

            full_scramble_length: hyperpuzzle_core::FULL_SCRAMBLE_LENGTH,
        })
    }

    /// Returns the nubmer of dimensions of the underlying space the puzzle is
    /// built in. Equivalent to `self.space.ndim()`.
    pub fn ndim(&self) -> u8 {
        self.space.ndim()
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
        let opt_id = Some(self.meta.id.as_str());

        let shape_builder = self.shape.lock();
        let twists_builder = self.twists.lock();
        let space = &shape_builder.space;
        let ndim = space.ndim();

        // Build color system. TODO: cache this if unmodified
        let colors = Arc::new(shape_builder.colors.build(build_ctx, opt_id, warn_fn)?);

        // Build twist system. TODO: cache this if unmodified
        let twists = Arc::new(twists_builder.build(build_ctx, opt_id, warn_fn)?);

        if let Some(build_ctx) = build_ctx {
            build_ctx.progress.lock().task = BuildTask::BuildingPuzzle;
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

        let engine_data = twists
            .engine_data
            .downcast_ref::<NdEuclidTwistSystemEngineData>()
            .ok_or_eyre("expected NdEuclid twist system")?;
        let NdEuclidTwistSystemEngineData {
            axis_vectors,
            twist_transforms,
            ..
        } = engine_data;

        // Build twist gizmos.
        let gizmo_twists =
            super::gizmos::build_twist_gizmos(space, &mut mesh, &twists, engine_data, warn_fn)?;

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

        let geom = Arc::new(NdEuclidPuzzleGeometry {
            vertex_coordinates,
            piece_vertex_sets,
            piece_centroids,

            planes,
            sticker_planes,

            mesh,

            axis_vectors: Arc::clone(axis_vectors),
            twist_transforms: Arc::clone(twist_transforms),

            gizmo_twists,
        });
        let ui_data = NdEuclidPuzzleUiData::new_dyn(&geom);

        // Build layers.
        let mut axis_layers = self.axis_layers.clone();
        axis_layers.resize(twists.axes.len())?;
        let axis_layers = axis_layers.try_map_ref(|_, layers| layers.build())?;

        let mut scramble_twists = twists
            .twists
            .iter_filter(|_, twist_info| {
                twist_info.include_in_scrambles && !axis_layers[twist_info.axis].is_empty()
            })
            .collect_vec();
        scramble_twists.sort_by_cached_key(|&twist| match twists.names.get(twist) {
            Ok(name) => &name.canonical,
            Err(_) => "",
        });
        let can_scramble = !scramble_twists.is_empty();

        let axis_layers_info = axis_layers.map_ref(|_, depths| AxisLayersInfo {
            max_layer: depths.len() as u16,
            allow_negatives: false, // TODO: configurable negative layers
        });

        let random_move = Box::new({
            let twists = Arc::clone(&twists);
            let axis_layers_info = axis_layers_info.clone();
            move |rng: &mut dyn Rng| {
                let random_twist = *scramble_twists.choose(rng)?;

                let axis = twists.twists[random_twist].axis;
                let layer_count = axis_layers_info[axis].max_layer;
                let random_layer_mask = if layer_count == 0 {
                    log::error!("Selected scramble twist axis has no layers");
                    None // shouldn't be possible
                } else {
                    let mut random_bits = std::iter::from_fn(|| Some(rng.next_u32()))
                        .flat_map(|bits: u32| (0..u32::BITS).map(move |i| bits & (1 << i) != 0));
                    std::iter::from_fn(|| {
                        LayerRange::all(layer_count)
                            .filter(|_| random_bits.next().expect("end of random bits"))
                            .map(LayerMask::from_iter)
                    })
                    .find(|mask| !mask.is_empty())
                };

                Some(Move {
                    layers: random_layer_mask.unwrap_or_default().into(), // should always be `Some`
                    transform: hypuz_notation::Transform::new(&twists.names[random_twist], None),
                    multiplier: hypuz_notation::Multiplier(1),
                })
            }
        });

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

            axis_layers_info,
            axis_layers,
            twists,

            ui_data,

            new: Box::new(move |this| NdEuclidPuzzleState::new(this, Arc::clone(&geom)).into()),

            random_move,
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

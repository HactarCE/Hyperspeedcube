use std::collections::HashMap;
use std::sync::{Arc, Weak};

use eyre::{OptionExt, Result, eyre};
use hypermath::prelude::*;
use hyperpuzzle_core::catalog::{BuildCtx, BuildTask};
use hyperpuzzle_core::prelude::*;
use hypershape::prelude::*;
use itertools::Itertools;
use parking_lot::Mutex;

use super::shape::ShapeBuildOutput;
use super::{AxisLayersBuilder, ShapeBuilder, TwistSystemBuilder};
use crate::NdEuclidTwistSystemEngineData;
use crate::prelude::*;

/// Puzzle being constructed.
#[derive(Debug)]
pub struct PuzzleBuilder {
    /// Reference-counted pointer to this struct.
    pub this: Weak<Mutex<Self>>,

    /// Puzzle metadata.
    pub meta: PuzzleListMetadata,

    /// Shape of the puzzle.
    pub shape: ShapeBuilder,
    /// Twist system of the puzzle.
    pub twists: TwistSystemBuilder,

    /// Layer data for each layer on the axis, in order from outermost to
    /// innermost.
    ///
    /// This is private because we must resize it whenever it is accessed to
    /// ensure it's the same length as `twists.axes`.
    axis_layers: PerAxis<AxisLayersBuilder>,

    /// Number of moves for a full scramble.
    pub full_scramble_length: u32,
}
impl PuzzleBuilder {
    /// Constructs a new puzzle builder with a primordial cube.
    pub fn new(meta: PuzzleListMetadata, ndim: u8) -> Result<Arc<Mutex<Self>>> {
        let shape = ShapeBuilder::new_with_primordial_cube(Space::new(ndim), &meta.id)?;
        let twists = TwistSystemBuilder::new();
        Ok(Arc::new_cyclic(|this| {
            Mutex::new(Self {
                this: this.clone(),

                meta,

                shape,
                twists,

                axis_layers: PerAxis::new(),

                full_scramble_length: hyperpuzzle_core::FULL_SCRAMBLE_LENGTH,
            })
        }))
    }

    /// Returns an `Arc` reference to the puzzle builder.
    pub fn arc(&self) -> Arc<Mutex<Self>> {
        self.this
            .upgrade()
            .expect("`PuzzleBuilder` removed from `Arc`")
    }

    /// Returns the nubmer of dimensions of the underlying space the puzzle is
    /// built in. Equivalent to `self.shape.lock().space.ndim()`.
    pub fn ndim(&self) -> u8 {
        self.shape.space.ndim()
    }
    /// Returns the underlying space the puzzle is built in. Equivalent to
    /// `self.shape.lock().space`
    pub fn space(&self) -> Arc<Space> {
        Arc::clone(&self.shape.space)
    }

    /// Returns a mutable reference to the axis layers.
    pub fn axis_layers(&mut self) -> Result<&mut PerAxis<AxisLayersBuilder>> {
        self.axis_layers.resize(self.twists.axes.len())?;
        Ok(&mut self.axis_layers)
    }

    /// Performs the final steps of building a puzzle, generating the mesh and
    /// assigning IDs to pieces, stickers, etc.
    pub fn build(
        &self,
        build_ctx: Option<&BuildCtx>,
        warn_fn: impl Copy + Fn(eyre::Error),
    ) -> Result<Arc<Puzzle>> {
        let opt_id = Some(self.meta.id.as_str());

        // Build color system. TODO: cache this if unmodified
        let colors = Arc::new(self.shape.colors.build(build_ctx, opt_id, warn_fn)?);

        // Build twist system. TODO: cache this if unmodified
        let twists = Arc::new(self.twists.build(build_ctx, opt_id, warn_fn)?);

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
        } = self.shape.build(warn_fn)?;

        let mut scramble_twists = twists
            .twists
            .iter_filter(|_, twist_info| twist_info.include_in_scrambles)
            .collect_vec();
        scramble_twists.sort_by_cached_key(|&twist| match twists.names.get(twist) {
            Ok(name) => &name.canonical,
            Err(_) => "",
        });

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
        let gizmo_twists = super::gizmos::build_twist_gizmos(
            &self.space(),
            &mut mesh,
            &twists,
            engine_data,
            warn_fn,
        )?;

        // Build vertex sets.
        let space = self.space();
        let mut vertex_count = 0;
        let mut vertex_coordinates = vec![];
        let mut vertex_id_map = HashMap::new();
        let piece_vertex_sets = piece_polytopes.map(|_piece, polytope_id| {
            space
                .get(polytope_id)
                .vertex_set()
                .map(|v| {
                    *vertex_id_map.entry(v.id()).or_insert_with(|| {
                        vertex_coordinates.extend(v.pos().iter_ndim(space.ndim()));
                        let i = vertex_count;
                        vertex_count += 1;
                        i
                    })
                })
                .collect()
        });

        // Build hyperplanes.
        let mut planes = vec![];
        let mut plane_id_map = ApproxHashMap::new();
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

        // Assign opposite axes.
        let mut axis_opposites: PerAxis<Option<Axis>> = PerAxis::new();
        for axis in Axis::iter(twists.axes.len()) {
            if axis_opposites[axis].is_some() {
                continue; // already visited it
            }

            if let Some(opposite_axis) = self.twists.axes.vector_to_id(-&axis_vectors[axis]) {
                let self_layers = &axis_layers[axis].0;
                let opposite_layers = &axis_layers[opposite_axis].0;

                // Do the layers overlap?
                let overlap = Option::zip(self_layers.last(), opposite_layers.last())
                    .is_some_and(|(l1, l2)| l1.bottom < -l2.bottom);

                if overlap {
                    // Are the layers exactly the same, just reversed?
                    let is_same_but_reversed = self_layers.len() == opposite_layers.len()
                        && std::iter::zip(
                            self_layers.iter_values().rev(),
                            opposite_layers.iter_values(),
                        )
                        .all(|(l1, l2)| {
                            approx_eq(&l1.top, &-l2.bottom) && approx_eq(&l1.bottom, &-l2.top)
                        });

                    if is_same_but_reversed {
                        axis_opposites[axis] = Some(opposite_axis);
                        axis_opposites[opposite_axis] = Some(axis);
                    } else {
                        let name1 = &twists.axes.names[axis];
                        let name2 = &twists.axes.names[opposite_axis];
                        let layers1 = &axis_layers[axis];
                        let layers2 = &axis_layers[opposite_axis];
                        warn_fn(eyre!(
                            "axes {name1} and {name2} are opposite and overlapping, \
                             but the layers do not match ({layers1} vs. {layers2})"
                        ));
                    }
                }
            }
        }

        Ok(Arc::new_cyclic(|this| Puzzle {
            this: Weak::clone(this),
            meta: self.meta.clone(),

            view_prefs_set: Some(PuzzleViewPreferencesSet::Perspective(
                PerspectiveDim::from_ndim(self.ndim()),
            )),

            pieces,
            stickers,
            piece_types,
            piece_type_hierarchy,
            piece_type_masks,

            colors,

            scramble_twists,
            full_scramble_length: self.full_scramble_length,

            notation: Notation {},

            axis_layers,
            axis_opposites,
            twists,

            ui_data,

            new: Box::new(move |this| NdEuclidPuzzleState::new(this, Arc::clone(&geom)).into()),
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
    cached_interior_point: Option<Vector>,
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

    pub(super) fn interior_point(&mut self, space: &Space) -> &Vector {
        // Average the vertices to get a point that is inside the polytope. For
        // polytopes with many vertices, this could perhaps be improved by using
        // blades.
        self.cached_interior_point.get_or_insert_with(|| {
            let mut count = 0;
            let mut sum = vector![];
            for v in space.get(self.polytope).vertex_set() {
                count += 1;
                sum += v.pos();
            }
            sum / count as _
        })
    }
}

/// Piece type of a puzzle during puzzle construction.
#[derive(Debug, Clone)]
pub struct PieceTypeBuilder {
    #[allow(clippy::doc_markdown)]
    /// Name for the piece type. (e.g., "center/oblique_1_2/left")
    pub name: String,
}

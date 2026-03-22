//! Symmetric Euclidean puzzle simulation backend and Hyperpuzzlescript API for
//! Hyperspeedcube.

use std::collections::HashMap;
use std::sync::{Arc, Weak};

use eyre::{OptionExt, Result, ensure, eyre};
use hypermath::prelude::*;
use hyperpuzzle_core::group::{GroupAction, IsometryGroup};
use hyperpuzzle_core::prelude::*;
use hyperpuzzle_impl_nd_euclid::builder::ColorSystemBuilder;
use hyperpuzzle_impl_nd_euclid::{NdEuclidPuzzleGeometry, NdEuclidPuzzleUiData};

mod axes;
mod from_space;
mod piece;
mod surface;

use itertools::Itertools;
use parking_lot::Mutex;
use piece::{PieceBuilder, PieceFacetBuilder, StickerBuilder};
use surface::SurfaceBuilder;

use crate::names::NameBiMap;
use crate::{ProductPuzzleState, SymmetricTwistSystemEngineData};
use axes::{AxisOrbitBuilder, AxisSetBuilder};

#[derive(Debug)]
pub struct ProductPuzzleBuilder {
    ndim: u8,

    /// Pieces and stickers.
    pieces: PerPiece<PieceBuilder>,
    /// Surfaces.
    surfaces: PerSurface<SurfaceBuilder>,
    /// Colors
    colors: PerColor<()>,

    axis_sets: Vec<AxisSetBuilder>,
    axis_vectors: PerAxis<Vector>,
    axis_group_action: GroupAction<Axis>,

    sticker_color_action: GroupAction<Color>,
    sticker_color_names: NameBiMap<Color>,

    isometry_group: IsometryGroup,
    // /// Twist gizmos to generate, which will be expanded by the symmetry of the
    // /// puzzle.
    // twist_gizmo_seeds: Vec<TwistGizmoBuilder>,
}

impl ProductPuzzleBuilder {
    /// Returns the number of the dimensions of the puzzle.
    pub fn ndim(&self) -> u8 {
        self.ndim
    }

    /// Returns the number of pieces in the puzzle.
    pub fn piece_count(&self) -> usize {
        self.pieces.len()
    }
    /// Returns the number of surfaces on the puzzle.
    pub fn surface_count(&self) -> usize {
        self.surfaces.len()
    }
    /// Returns the number of sticker colors on the puzzle.
    pub fn color_count(&self) -> usize {
        self.colors.len()
    }
    /// Returns the number of axes on the puzzle.
    pub fn axis_count(&self) -> usize {
        self.axis_sets.iter().map(|axis_set| axis_set.len).sum()
    }

    /// Constructs the empty puzzle, which is the identity of the direct
    /// product.
    pub fn direct_product_identity() -> Self {
        ProductPuzzleBuilder {
            ndim: 0,
            pieces: PerPiece::from_iter([PieceBuilder::POINT]),
            surfaces: PerSurface::new(),
            colors: PerColor::new(),
            axis_sets: vec![],
            axis_vectors: PerAxis::new(),
            axis_group_action: GroupAction::trivial(),
            sticker_color_action: GroupAction::trivial(),
            sticker_color_names: NameBiMap::new(),
            isometry_group: IsometryGroup::trivial(),
        }
    }

    /// Constructs a symmetric puzzle.
    ///
    /// Each axis has a vector and a list of depths, which must be sorted from
    /// outermost (greatest) to innermost (least).
    pub fn new_ft(
        ndim: u8,
        symmetry: IsometryGroup,
        axes: &[(Vector, Vec<Float>)],
    ) -> Result<Self> {
        let generator_motors = symmetry
            .generator_motors()
            .into_iter()
            .map(|(g, motor)| (hypergroup::GenSeq::new([g]), motor.clone()))
            .collect_vec();

        let carve_planes = axes
            .iter()
            .filter_map(|(v, _depths)| Hyperplane::from_pole(v));
        let mut slice_cuts = vec![];

        let mut axis_vectors = PerAxis::new();
        let mut axis_names = NameBiMap::new();
        let mut axis_orbits = vec![];
        let mut autonames = crate::autonames(); // TODO: proper names
        for (axis_vector, depths) in axes {
            ensure!(
                depths.is_sorted_by(|a, b| a > b),
                "depths {depths:?} are not sorted from outermost (greatest) to innermost (least)",
            );

            let axis_iter_start = axis_names.len();

            let mut gen_seqs = vec![];
            for (gen_seq, _motor, vector) in
                hypergroup::orbit_geometric_with_gen_seq(&generator_motors, axis_vector.clone())
            {
                gen_seqs.push(gen_seq);
                axis_vectors.push(vector.clone())?; // TODO: what about overlapping axes?
                axis_names.push(autonames.next().unwrap())?;
            }

            let axis_iter_end = axis_names.len();

            let layers = Layer::iter(depths.len() - 1).map(Some).chain([None]); // last cut has no layer
            for (layer, &depth) in layers.zip(depths) {
                slice_cuts.push((axis_iter_start..axis_iter_end, depth, layer));
            }
            axis_orbits.push(AxisOrbitBuilder {
                len: gen_seqs.len(),
                vector: axis_vector.clone(),
                max_layer: (depths.len() - 1)
                    .try_into()
                    .map_err(|_| eyre!("too many layers"))?,
                generator_sequences: Arc::new(gen_seqs),
            });
        }

        let axis_set = AxisSetBuilder {
            ndim,
            len: axis_vectors.len(),
            id_offset: 0,
            names: Arc::new(axis_names),
            orbits: axis_orbits,
        };

        let mut shape_builder = from_space::PuzzleShapeBuilder::new(ndim, symmetry, axis_set.len)?;
        for plane in carve_planes {
            shape_builder.carve_symmetric(&plane)?;
        }
        shape_builder.set_surface_centroids_from_stickers_of_single_piece(Piece(0))?;
        for (axis_iter, distance, layer) in slice_cuts {
            shape_builder.slice_symmetric(
                axis_iter
                    .map(|i| Axis(i as u16))
                    .map(|ax| (ax, &axis_vectors[ax])),
                distance,
                layer,
            )?;
        }
        let mut ret = shape_builder.into_product_puzzle_builder()?;

        let axis_points = axis_vectors.map_ref(|_, v| Point(v.clone()));
        ret.axis_sets = vec![axis_set];
        ret.axis_vectors = axis_vectors;
        ret.axis_group_action = ret.isometry_group.action_on_points(&axis_points)?;

        Ok(ret)
    }

    /// Returns the direct product of two puzzles.
    ///
    /// The direct product of two puzzles `a` and `b` will have dimension
    /// `a.ndim() + b.ndim()`, with puzzle `a` occupying the lower dimensions
    /// and puzzle `b` occupying the higher dimensions.
    pub fn direct_product(&self, rhs: &Self) -> Result<Self> {
        let a = self;
        let b = rhs;

        let ndim = a.ndim + b.ndim;
        let a_axis_count = a.axis_count();

        let pieces = itertools::iproduct!(a.pieces.iter_values(), b.pieces.iter_values(),)
            .map(|(a_piece, b_piece)| {
                PieceBuilder::direct_product(a_piece, b_piece, a.surface_count(), a.color_count())
            })
            .collect();

        // Assume that the centroid of each entire puzzle is the origin.
        let surfaces = std::iter::chain(
            a.surfaces
                .iter_values()
                .map(|a_surface| a_surface.lift_by_ndim(0, b.ndim)),
            b.surfaces
                .iter_values()
                .map(|b_surface| b_surface.lift_by_ndim(a.ndim, 0)),
        )
        .collect();

        let colors = std::iter::chain(a.colors.iter_values(), b.colors.iter_values())
            .copied()
            .collect();

        let axis_sets = std::iter::chain(
            a.axis_sets
                .iter()
                .map(|a_axis_set| a_axis_set.lift_ndim(0, b.ndim)),
            b.axis_sets
                .iter()
                .map(|b_axis_set| b_axis_set.lift_ndim(a.ndim, 0).offset_ids_by(a_axis_count)),
        )
        .collect();

        let axis_vectors = std::iter::chain(
            a.axis_vectors
                .iter_values()
                .map(|v| crate::lift_vector_by_ndim(v, 0, a.ndim, b.ndim)),
            b.axis_vectors
                .iter_values()
                .map(|v| crate::lift_vector_by_ndim(v, a.ndim, b.ndim, 0)),
        )
        .collect();

        Ok(ProductPuzzleBuilder {
            ndim,

            pieces,
            surfaces,
            colors,

            axis_group_action: GroupAction::product([&a.axis_group_action, &b.axis_group_action])?,
            axis_sets,
            axis_vectors,

            sticker_color_action: GroupAction::product([
                &a.sticker_color_action,
                &b.sticker_color_action,
            ])?,
            sticker_color_names: NameBiMap::concat(&a.sticker_color_names, &b.sticker_color_names),

            isometry_group: IsometryGroup::product([&a.isometry_group, &b.isometry_group])?,
        })
    }

    /// Constructs the final puzzle.
    pub fn build(&self) -> Result<Arc<Puzzle>> {
        let ndim = self.ndim;
        let piece_count = self.piece_count();

        // Build pieces and stickers.
        let mut stickers = PerSticker::new();
        let pieces = self.pieces.try_map_ref(|piece, piece_builder| {
            let stickers = piece_builder
                .facets
                .iter()
                .filter_map(|f| f.sticker_data.as_ref())
                .map(|sticker_data| {
                    stickers.push(StickerInfo {
                        piece,
                        color: sticker_data.color,
                    })
                })
                .try_collect()?;
            eyre::Ok(PieceInfo {
                stickers,
                piece_type: PieceType(0),
            })
        })?;

        let geom = Arc::new(NdEuclidPuzzleGeometry {
            vertex_coordinates: vec![],
            piece_vertex_sets: PerPiece::new_with_len(piece_count),
            piece_centroids: self
                .pieces
                .map_ref(|_, piece_geometries| piece_geometries.polytope.centroid.center()),

            planes: vec![Hyperplane::new(vector![1.0], 0.0).unwrap()],
            sticker_planes: stickers.map_ref(|_, _| 0),

            mesh: self.build_mesh()?,

            axis_vectors: Arc::new(PerAxis::new()),
            axis_layer_depths: PerAxis::new(),
            twist_transforms: Arc::new(PerTwist::new()),

            gizmo_twists: PerGizmoFace::new(),
        });
        let ui_data = NdEuclidPuzzleUiData::new_dyn(&geom);

        // TODO: proper color system
        let mut colors = ColorSystemBuilder::new_ad_hoc("unknown_product_puzzle");
        for _ in &self.surfaces {
            colors.add(None, |e| log::warn!("{e}"))?;
        }
        let colors = Arc::new(colors.build(None, None, &mut |e| log::warn!("{e}"))?);

        let piece_types = PerPieceType::from_iter([PieceTypeInfo {
            name: "piece".to_string(),
            display: "Piece".to_ascii_lowercase(),
        }]);
        let mut piece_type_hierarchy = PieceTypeHierarchy::new(6);
        for (id, piece_type_info) in &piece_types {
            if let Err(e) = piece_type_hierarchy.set_piece_type_id(&piece_type_info.name, id) {
                log::warn!("{e}");
            }
        }

        let piece_type_masks =
            HashMap::from_iter([("piece".to_string(), PieceMask::new_full(piece_count))]);

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
        let mut twists = TwistSystem::new_empty(&axes);
        let twist_system_engine_data = Arc::new(SymmetricTwistSystemEngineData {
            axes,
            axis_vectors,
            group: self.isometry_group.clone(),
            group_action: self.axis_group_action.clone(),
            constraint_solver: Arc::new(Mutex::new(hypergroup::ConstraintSolver::new(
                self.axis_group_action.clone(),
            ))),
        });
        twists.engine_data = (*twist_system_engine_data).clone().into();
        let twists = Arc::new(twists);

        let axis_layers = self
            .axis_sets
            .iter()
            .flat_map(|axis_set| {
                axis_set.orbits.iter().flat_map(|axis_orbit| {
                    std::iter::repeat_n(
                        AxisLayersInfo {
                            max_layer: axis_orbit.max_layer,
                            allow_negatives: false, // TODO: allow negatives on some axes
                        },
                        axis_orbit.len,
                    )
                })
            })
            .collect();

        let grip_signatures = Arc::new(
            Axis::iter(twists.axes.len())
                .map(|axis| self.pieces.map_ref(|_, piece| piece.grip_signature[axis]))
                .collect(),
        );

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
            random_move: Box::new(|rng| None),
        }))
    }

    /// Constructs a mesh for rendering the puzzle.
    fn build_mesh(&self) -> Result<Mesh> {
        let mut mesh = Mesh::new_empty(self.ndim);

        // Add puzzle surfaces to the mesh with the same IDs as they have in
        // `self.surfaces`.
        for (_surface, surface_geometry) in &self.surfaces {
            mesh.add_puzzle_surface(&surface_geometry.centroid, &surface_geometry.normal)?;
        }
        let dummy_surface = mesh.add_puzzle_surface(&Point::ORIGIN, Vector::EMPTY)?; // dummy surface for internals and 2D puzzles

        // Add pieces to the mesh.
        for (_piece, piece_builder) in &self.pieces {
            piece_builder.add_to_mesh(&mut mesh, dummy_surface)?;
        }

        Ok(mesh)
    }
}

pub struct TwistGizmoBuilder {
    /// Distance from the origin (3D) or axis vector (4D).
    pub distance: Float,
    /// Clockwise twist for the gizmo.
    pub twist: Twist,
}

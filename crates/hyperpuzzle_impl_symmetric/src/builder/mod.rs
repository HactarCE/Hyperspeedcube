//! Symmetric Euclidean puzzle simulation backend and Hyperpuzzlescript API for
//! Hyperspeedcube.

use std::collections::HashMap;
use std::sync::{Arc, Weak};

use eyre::Result;
use hypermath::prelude::*;
use hyperpuzzle_core::group::{GroupAction, IsometryGroup};
use hyperpuzzle_core::prelude::*;
use hyperpuzzle_impl_nd_euclid::builder::ColorSystemBuilder;
use hyperpuzzle_impl_nd_euclid::{NdEuclidPuzzleGeometry, NdEuclidPuzzleUiData};

mod from_space;
mod piece;
mod surface;

use itertools::Itertools;
use piece::{PieceBuilder, PieceFacetBuilder, StickerBuilder};
use surface::SurfaceBuilder;

use crate::names::NameBiMap;
use crate::{NamedPoint, ProductPuzzle};

#[derive(Debug)]
pub struct ProductPuzzleBuilder {
    ndim: u8,

    /// Pieces and stickers.
    pieces: PerPiece<PieceBuilder>,
    /// Surfaces.
    surfaces: PerSurface<SurfaceBuilder>,
    /// Colors
    colors: PerColor<()>,

    named_point_group_action: GroupAction<NamedPoint>,
    named_point_names: NameBiMap<NamedPoint>,

    axis_group_action: GroupAction<Axis>,
    axis_names: NameBiMap<Axis>,

    sticker_color_action: GroupAction<Color>,
    sticker_color_names: NameBiMap<Color>,

    isometry_group: IsometryGroup,
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

    /// Constructs the empty puzzle, which is the identity of the direct
    /// product.
    pub fn direct_product_identity() -> Self {
        ProductPuzzleBuilder {
            ndim: 0,
            pieces: PerPiece::from_iter([PieceBuilder::POINT]),
            surfaces: PerSurface::new(),
            colors: PerColor::new(),
            named_point_group_action: GroupAction::trivial(),
            named_point_names: NameBiMap::new(),
            axis_group_action: GroupAction::trivial(),
            axis_names: NameBiMap::new(),
            sticker_color_action: GroupAction::trivial(),
            sticker_color_names: NameBiMap::new(),
            isometry_group: IsometryGroup::trivial(),
        }
    }

    /// Constructs a symmetric puzzle.
    pub fn new(
        ndim: u8,
        symmetry: IsometryGroup,
        carve_planes: &[Hyperplane],
        slice_planes: &[Hyperplane],
    ) -> Result<Self> {
        let mut builder = from_space::PuzzleBuilderFromHypershape::new(ndim, symmetry)?;
        for plane in carve_planes {
            builder.carve_symmetric(plane)?;
        }
        builder.set_surface_centroids_from_stickers_of_single_piece(Piece(0))?;
        for plane in slice_planes {
            builder.slice_symmetric(plane)?;
        }
        builder.into_product_puzzle_builder()
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

        Ok(ProductPuzzleBuilder {
            ndim,

            pieces,
            surfaces,
            colors,

            named_point_group_action: GroupAction::product([
                &a.named_point_group_action,
                &b.named_point_group_action,
            ])?,
            named_point_names: NameBiMap::concat(&a.named_point_names, &b.named_point_names),

            axis_group_action: GroupAction::product([&a.axis_group_action, &b.axis_group_action])?,
            axis_names: NameBiMap::concat(&a.axis_names, &b.axis_names),

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

        let axes = Arc::new(AxisSystem::new_empty());
        let twists = Arc::new(TwistSystem::new_empty(&axes));

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
            axis_layers: PerAxis::new(),
            twists,
            ui_data,
            new: Box::new({
                let grip_group = self.isometry_group.clone();
                move |ty| {
                    ProductPuzzle {
                        ty,
                        grip_group: grip_group.clone(),
                        attitudes: PerPiece::new_with_len(piece_count),
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

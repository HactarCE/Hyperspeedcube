//! Types for constructing pieces and piece facets, including stickers.

use eyre::Result;
use hypergroup::IsometryGroup;
use hypermath::prelude::*;
use hyperpuzzle_core::prelude::*;

use crate::geometry::PolytopeGeometry;

/// Shape of a puzzle under construction.
#[derive(Debug)]
pub(super) struct ProductPuzzleShape {
    /// Symmetry group of the puzzle shape.
    pub symmetry: IsometryGroup,
    /// Pieces and stickers.
    pub pieces: PerPiece<PieceData>,
    /// Surfaces.
    pub surfaces: PerSurface<SurfaceData>,
    /// Colors
    pub colors: PerColor<()>,
}

impl ProductPuzzleShape {
    /// Returns the number of dimensions of the puzzle and the space that
    /// contains it.
    pub fn ndim(&self) -> u8 {
        self.symmetry.ndim()
    }

    /// Constructs the empty puzzle shape, which is the identity of the direct
    /// product.
    pub fn direct_product_identity() -> Self {
        Self {
            symmetry: IsometryGroup::trivial(),
            pieces: PerPiece::from_iter([PieceData::POINT]),
            surfaces: PerSurface::new(),
            colors: PerColor::new(),
        }
    }

    /// Returns the direct product of two puzzle shapes.
    ///
    /// See [`super::ProductPuzzleBuilder::direct_product()`].
    pub fn direct_product(&self, rhs: &Self) -> Result<Self> {
        let a = self;
        let b = rhs;

        let pieces = itertools::iproduct!(a.pieces.iter_values(), b.pieces.iter_values(),)
            .map(|(a_piece, b_piece)| {
                PieceData::direct_product(a_piece, b_piece, a.surfaces.len(), a.colors.len())
            })
            .collect();

        // Assume that the centroid of each entire puzzle is the origin.
        let surfaces = std::iter::chain(
            a.surfaces
                .iter_values()
                .map(|a_surface| a_surface.lift_by_ndim(0, b.ndim())),
            b.surfaces
                .iter_values()
                .map(|b_surface| b_surface.lift_by_ndim(a.ndim(), 0)),
        )
        .collect();

        let colors = std::iter::chain(a.colors.iter_values(), b.colors.iter_values())
            .copied()
            .collect();

        Ok(Self {
            symmetry: IsometryGroup::product([&a.symmetry, &b.symmetry])?,
            pieces,
            surfaces,
            colors,
        })
    }

    /// Constructs a mesh for rendering the puzzle.
    pub fn build_mesh(&self) -> Result<Mesh> {
        let mut mesh = Mesh::new_empty(self.ndim());

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

/// Data for a piece in a puzzle under construction.
#[derive(Debug)]
pub(super) struct PieceData {
    /// Polytope of the piece, used to generate new stickers when computing the
    /// direct product of two pieces.
    ///
    /// This polytope is always N-dimensional, where N is the dimension of the
    /// space containing the puzzle.
    pub polytope: PolytopeGeometry,
    /// Facets of the piece, some of which may be stickers.
    ///
    /// In 3D and below, this includes non-sticker facets. In 4D+, non-sticker
    /// facets are removed because internals are never visible in 4D+.
    pub facets: Vec<PieceFacetData>,
    /// Grip signature for the piece.
    pub grip_signature: PerAxis<Option<Layer>>,
}

impl PieceData {
    /// Stickerless piece in a zero-dimensional space.
    pub const POINT: Self = Self {
        polytope: PolytopeGeometry::POINT,
        facets: vec![],
        grip_signature: PerAxis::new(),
    };

    /// Returns the direct product of two pieces.
    ///
    /// In order to track sticker data correctly, this requires the number of
    /// surfaces and colors in the `a` puzzle.
    pub fn direct_product(
        a: &Self,
        b: &Self,
        a_surface_count: usize,
        a_color_count: usize,
    ) -> Self {
        let ndim = a.polytope.space_ndim() + b.polytope.space_ndim();

        Self {
            polytope: PolytopeGeometry::direct_product(&a.polytope, &b.polytope),
            facets: std::iter::chain(
                a.facets
                    .iter()
                    .filter(|f| ndim <= 3 || f.sticker_data.is_some()) // remove internals in 4D+
                    .map(|a_facet| PieceFacetData {
                        polytope: PolytopeGeometry::direct_product(&a_facet.polytope, &b.polytope),
                        sticker_data: a_facet.sticker_data.as_ref().map(|sticker_data| {
                            StickerData {
                                surface: sticker_data.surface,
                                color: sticker_data.color,
                            }
                        }),
                    }),
                b.facets
                    .iter()
                    .filter(|f| ndim <= 3 || f.sticker_data.is_some()) // remove internals in 4D+
                    .map(|b_facet| PieceFacetData {
                        polytope: PolytopeGeometry::direct_product(&a.polytope, &b_facet.polytope),
                        sticker_data: b_facet.sticker_data.as_ref().map(|sticker_data| {
                            StickerData {
                                surface: Surface(a_surface_count as u16 + sticker_data.surface.0),
                                color: Color(a_color_count as u16 + sticker_data.color.0),
                            }
                        }),
                    }),
            )
            .collect(),
            grip_signature: std::iter::chain(
                a.grip_signature.iter_values().copied(),
                b.grip_signature.iter_values().copied(),
            )
            .collect(),
        }
    }

    /// Adds a piece to a mesh.
    ///
    /// `dummy_surface` is used for internals.
    pub fn add_to_mesh(&self, mesh: &mut Mesh, dummy_surface: Surface) -> Result<()> {
        let piece_id = Piece::try_from_index(mesh.piece_count)?;
        let centroid = self.polytope.centroid.center();

        // Add internals.
        let start = mesh.counts();
        if mesh.ndim == 2 {
            let interior_point = point![0.0, 0.0, -1.0]; // hack to orient 2D polygons correctly using 3D cross product
            self.polytope
                .add_to_mesh(mesh, dummy_surface, piece_id, &interior_point)?;
        } else if mesh.ndim == 3 {
            for facet in &self.facets {
                if facet.sticker_data.is_none() {
                    facet
                        .polytope
                        .add_to_mesh(mesh, dummy_surface, piece_id, &centroid)?;
                }
            }
        }
        let end = mesh.counts();
        mesh.add_piece(&centroid, start..end)?;

        // Add stickers.
        for facet in &self.facets {
            if let Some(sticker_data) = &facet.sticker_data {
                let sticker_range =
                    facet
                        .polytope
                        .add_to_mesh(mesh, sticker_data.surface, piece_id, &centroid)?;
                mesh.add_sticker(sticker_range)?;
            }
        }

        Ok(())
    }
}

/// Data for a facet of a piece under construction.
///
/// Facets may or may be stickers.
#[derive(Debug)]
pub(super) struct PieceFacetData {
    /// Polytope of the facet, used to generate mesh data.
    ///
    /// This polytope is always (N-1)-dimensional, where N is the dimension of
    /// the space containing the puzzle.
    pub polytope: PolytopeGeometry,
    /// Sticker data, if this facet is a sticker.
    ///
    /// This is `None` for internal facets, which are not displayed in higher
    /// dimensions.
    pub sticker_data: Option<StickerData>,
}

/// Additional data for a sticker facet under construction.
#[derive(Debug)]
pub(super) struct StickerData {
    /// Surface that the sticker is part of.
    pub surface: Surface,
    /// Color of the sticker, which typically (but not always) corresponds to
    /// the surface.
    pub color: Color,
}

#[derive(Debug)]
pub(super) struct SurfaceData {
    /// Number of dimensions of the space containing the surface.
    ///
    /// The surface is always one dimension lower than this.
    pub ndim: u8,
    /// Centroid of the surface, used to compute facet shrink.
    ///
    /// It is acceptable for this to be slightly inaccurate.
    pub centroid: Point,
    /// Normal vector to the surface, used to cull 4D backfaces.
    pub normal: Vector,
}

impl SurfaceData {
    pub fn lift_by_ndim(&self, ndim_below: u8, ndim_above: u8) -> Self {
        let centroid = self.centroid.as_vector();
        Self {
            ndim: ndim_below + self.ndim + ndim_above,
            centroid: crate::lift_vector_by_ndim(centroid, ndim_below, self.ndim, ndim_above),
            normal: crate::lift_vector_by_ndim(&self.normal, ndim_below, self.ndim, ndim_above),
        }
    }
}

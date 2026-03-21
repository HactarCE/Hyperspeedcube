//! Types for constructing pieces and piece facets, including stickers.

use eyre::Result;
use hypermath::prelude::*;
use hyperpuzzle_core::prelude::*;

use crate::geometry::PolytopeGeometry;

/// Info for a piece in a puzzle under construction.
#[derive(Debug)]
pub struct PieceBuilder {
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
    pub facets: Vec<PieceFacetBuilder>,
}

impl PieceBuilder {
    /// Stickerless piece in a zero-dimensional space.
    pub const POINT: Self = Self {
        polytope: PolytopeGeometry::POINT,
        facets: vec![],
    };

    /// Returns the number of dimensions of the piece.
    ///
    /// This is equal to the number of dimensions of the space containing the
    /// puzzle.
    pub fn ndim(&self) -> u8 {
        self.polytope.space_ndim()
    }

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
                    .map(|a_facet| PieceFacetBuilder {
                        polytope: PolytopeGeometry::direct_product(&a_facet.polytope, &b.polytope),
                        sticker_data: a_facet.sticker_data.as_ref().map(|sticker_data| {
                            StickerBuilder {
                                surface: sticker_data.surface,
                                color: sticker_data.color,
                            }
                        }),
                    }),
                b.facets
                    .iter()
                    .filter(|f| ndim <= 3 || f.sticker_data.is_some()) // remove internals in 4D+
                    .map(|b_facet| PieceFacetBuilder {
                        polytope: PolytopeGeometry::direct_product(&a.polytope, &b_facet.polytope),
                        sticker_data: b_facet.sticker_data.as_ref().map(|sticker_data| {
                            StickerBuilder {
                                surface: Surface(a_surface_count as u16 + sticker_data.surface.0),
                                color: Color(a_color_count as u16 + sticker_data.color.0),
                            }
                        }),
                    }),
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

/// Info for a facet of a piece under construction.
///
/// Facets may or may be stickers.
#[derive(Debug)]
pub struct PieceFacetBuilder {
    /// Polytope of the facet, used to generate mesh data.
    ///
    /// This polytope is always (N-1)-dimensional, where N is the dimension of
    /// the space containing the puzzle.
    pub polytope: PolytopeGeometry,
    /// Data about the sticker, if this facet is a sticker.
    ///
    /// This is `None` for internal facets, which are not displayed in higher
    /// dimensions.
    pub sticker_data: Option<StickerBuilder>,
}

/// Additional info for a sticker facet under construction.
#[derive(Debug)]
pub struct StickerBuilder {
    /// Surface that the sticker is part of.
    pub surface: Surface,
    /// Color of the sticker, which typically (but not always) corresponds to
    /// the surface.
    pub color: Color,
}

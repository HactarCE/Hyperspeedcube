//! Types for constructing pieces and piece facets, including stickers.

use std::{collections::HashMap, sync::Arc};

use eyre::{Result, eyre};
use hypergroup::IsometryGroup;
use hypermath::prelude::*;
use hyperpuzzle_core::{ComponentList, prelude::*};
use hypuz_notation::{Str, family::SequentialLowercaseName};
use indexmap::{IndexMap, IndexSet};
use itertools::Itertools;
use smallvec::SmallVec;

use crate::geometry::PolytopeGeometry;

/// Shape of a puzzle under construction.
#[derive(Debug, Clone)]
pub(super) struct ProductPuzzleShape {
    /// Symmetry group of the shape.
    pub group: IsometryGroup,
    /// Number of [`SequentialLowercaseName`] prefixes used.
    pub factor_count: usize,
    /// Pieces and stickers.
    pub pieces: PerPiece<PieceData>,
    /// Data for each surface.
    pub surfaces: PerSurface<SurfaceData>,
    /// Orbit data for colors.
    pub color_orbits: Vec<Orbit<DisjointUnionColorName>>,
}

impl ProductPuzzleShape {
    /// Returns the number of the dimensions of the puzzle.
    pub fn ndim(&self) -> u8 {
        self.group.ndim()
    }

    /// Constructs the empty puzzle shape, which is the identity of the direct
    /// product.
    pub fn direct_product_identity() -> Self {
        Self {
            group: IsometryGroup::trivial(),
            factor_count: 0,
            pieces: PerPiece::from_iter([PieceData::POINT]),
            surfaces: PerSurface::new(),
            color_orbits: vec![],
        }
    }

    /// Returns the direct product of two puzzle shapes.
    ///
    /// See [`super::ProductPuzzleBuilder::direct_product()`].
    pub fn direct_product(&self, rhs: &Self) -> Result<Self> {
        let a = self;
        let b = rhs;

        let pieces = itertools::iproduct!(a.pieces.iter_values(), b.pieces.iter_values(),)
            .map(|(a_piece, b_piece)| PieceData::direct_product(a_piece, b_piece, a.surfaces.len()))
            .collect();

        // Assume that the centroid of each entire puzzle is the origin.
        let surfaces = std::iter::chain(
            a.surfaces
                .iter_values()
                .map(|a_surface| a_surface.lift_by_ndim(0, 0, a.ndim(), b.ndim())),
            b.surfaces
                .iter_values()
                .map(|b_surface| b_surface.lift_by_ndim(a.factor_count, a.ndim(), b.ndim(), 0)),
        )
        .try_collect()?;

        Ok(Self {
            group: IsometryGroup::product([&a.group, &b.group])?,
            factor_count: a.factor_count + b.factor_count,
            pieces,
            surfaces,
            color_orbits: std::iter::chain(
                a.color_orbits.iter().cloned(),
                b.color_orbits
                    .iter()
                    .map(|orbit| orbit.map(|name| Some(name.offset(a.factor_count)))),
            )
            .collect(),
        })
    }

    pub fn build_ad_hoc_color_system(&self, puzzle_id: CatalogId) -> Result<Arc<ColorSystem>> {
        let id = hyperpuzzle_core::ad_hoc_id(puzzle_id);
        let name = id.to_string();
        let needs_prefix = self.factor_count != 1;

        let names = Names::new_simple(
            self.surfaces
                .iter_values()
                .map(|data| data.color.to_str(needs_prefix))
                .collect::<IndexSet<Str>>()
                .into_iter()
                .collect(),
        )?;

        let scheme_name = hyperpuzzle_core::DEFAULT_COLOR_SCHEME_NAME;
        let scheme = ColorSystem::new_rainbow_scheme(names.len());

        let orbits = self
            .color_orbits
            .iter()
            .map(|orbit| orbit.map(|s| names.lookup(&s.to_str(needs_prefix))))
            .collect();

        Ok(Arc::new(ColorSystem {
            id,
            name,
            components: ComponentList::new(),
            names,
            schemes: IndexMap::from_iter([(scheme_name.to_string(), scheme)]),
            default_scheme: scheme_name.to_string(),
            orbits,
        }))
    }

    pub fn build_piece_and_stickers(
        &self,
        color_system: &ColorSystem,
    ) -> Result<(PerPiece<PieceInfo>, PerSticker<StickerInfo>)> {
        let needs_prefix = self.factor_count != 1;

        let mut pieces = PerPiece::new();
        let mut stickers = PerSticker::new();
        for (piece, piece_builder) in &self.pieces {
            let mut piece_stickers = SmallVec::with_capacity(piece_builder.sticker_count());
            for facet in piece_builder.facets.iter() {
                if let Some(sticker_data) = &facet.sticker_data {
                    let color_name = self.surfaces[sticker_data.surface]
                        .color
                        .to_str(needs_prefix);
                    let color = color_system
                        .names
                        .lookup(&color_name)
                        .ok_or_else(|| eyre!("no color named {color_name}"))?;
                    let sticker = stickers.push(StickerInfo { piece, color })?;
                    piece_stickers.push(sticker);
                }
            }
            pieces.push(PieceInfo {
                stickers: piece_stickers,
                piece_type: PieceType(0),
            })?;
        }

        Ok((pieces, stickers))
    }

    pub fn build_piece_centroids(&self) -> PerPiece<Point> {
        self.pieces
            .map_ref(|_, piece_geometries| piece_geometries.polytope.centroid.center())
    }

    pub fn build_piece_types(
        &self,
        warn_fn: &mut impl FnMut(eyre::Report),
    ) -> Result<(
        PerPieceType<PieceTypeInfo>,
        PieceTypeHierarchy,
        HashMap<String, PieceMask>,
    )> {
        let piece_types = PerPieceType::from_iter([PieceTypeInfo {
            name: "piece".to_string(),
            display: "Piece".to_ascii_lowercase(),
        }]);
        let mut piece_type_hierarchy = PieceTypeHierarchy::new(6);
        for (id, piece_type_info) in &piece_types {
            if let Err(e) = piece_type_hierarchy.set_piece_type_id(&piece_type_info.name, id) {
                warn_fn(e);
            }
        }

        let piece_type_masks =
            HashMap::from_iter([("piece".to_string(), PieceMask::new_full(self.pieces.len()))]);

        Ok((piece_types, piece_type_hierarchy, piece_type_masks))
    }

    /// Returns the grip signature for each piece.
    pub fn build_grip_signatures(&self) -> PerPiece<PerAxis<Option<LayerRange>>> {
        self.pieces.map_ref(|_, piece| piece.grip_signature.clone())
    }

    /// Constructs a mesh for rendering the puzzle.
    pub fn build_mesh(&self) -> Result<Mesh> {
        let mut mesh = Mesh::new_empty(self.ndim());

        // Add puzzle surfaces to the mesh with the same IDs as they have in
        // `self.surfaces`.
        for (_surface, surface_data) in &self.surfaces {
            mesh.add_puzzle_surface(&surface_data.centroid, surface_data.hyperplane.normal())?;
        }
        let dummy_surface = mesh.add_puzzle_surface(&Point::ORIGIN, Vector::EMPTY)?; // dummy surface for internals and 2D puzzles

        // Add pieces to the mesh.
        for (_piece, piece_builder) in &self.pieces {
            piece_builder.add_to_mesh(&mut mesh, dummy_surface)?;
        }

        Ok(mesh)
    }

    /// Constructs a list of unique surface hyperplanes and an index into that
    /// list for each sticker.
    pub fn build_sticker_planes(&self) -> (Vec<Hyperplane>, PerSticker<usize>) {
        (
            self.surfaces
                .iter_values()
                .map(|surface_data| surface_data.hyperplane.clone())
                .collect(),
            self.pieces
                .iter_values()
                .flat_map(|piece_data| &piece_data.facets)
                .filter_map(|piece_facet_data| piece_facet_data.sticker_data.as_ref())
                .map(|sticker_data| sticker_data.surface.to_index())
                .collect(),
        )
    }

    pub fn remove_internals(&mut self) {
        self.pieces = self
            .pieces
            .iter_values()
            .filter(|p| p.sticker_count() > 0)
            .cloned()
            .collect();
    }
}

/// Data for a piece in a puzzle under construction.
#[derive(Debug, Clone)]
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
    pub grip_signature: PerAxis<Option<LayerRange>>,
}

impl PieceData {
    /// Stickerless piece in a zero-dimensional space.
    pub const POINT: Self = Self {
        polytope: PolytopeGeometry::POINT,
        facets: vec![],
        grip_signature: PerAxis::new(),
    };

    /// Returns the number of stickers on the piece.
    pub fn sticker_count(&self) -> usize {
        self.facets
            .iter()
            .filter(|f| f.sticker_data.is_some())
            .count()
    }

    /// Returns the direct product of two pieces.
    ///
    /// In order to track sticker data correctly, this requires the number of
    /// surfaces and colors in the `a` puzzle.
    pub fn direct_product(a: &Self, b: &Self, a_surface_count: usize) -> Self {
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
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
pub(super) struct StickerData {
    /// Surface that the sticker is part of.
    ///
    /// This determines the color of the sticker.
    pub surface: Surface,
}

#[derive(Debug, Clone)]
pub(super) struct SurfaceData {
    /// Centroid of the surface, used to compute facet shrink.
    ///
    /// It is acceptable for this to be slightly inaccurate.
    pub centroid: Point,
    /// Hyperplane for the surface, whose normal vector is used to cull 4D
    /// backfaces.
    pub hyperplane: Hyperplane,
    /// Name of the sticker color for the surface.
    pub color: DisjointUnionColorName,
}

impl SurfaceData {
    pub fn lift_by_ndim(
        &self,
        color_prefix_offset: usize,
        ndim_below: u8,
        ndim: u8,
        ndim_above: u8,
    ) -> Result<Self> {
        let centroid = self.centroid.as_vector();
        Ok(Self {
            centroid: crate::lift_vector_by_ndim(centroid, ndim_below, ndim, ndim_above),
            hyperplane: crate::lift_hyperplane_by_ndim(
                &self.hyperplane,
                ndim_below,
                ndim,
                ndim_above,
            )?,
            color: self.color.offset(color_prefix_offset),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct DisjointUnionColorName {
    pub prefix: SequentialLowercaseName,
    pub name: Str,
}

impl DisjointUnionColorName {
    #[must_use]
    pub fn offset(&self, color_prefix_offset: usize) -> Self {
        Self {
            prefix: SequentialLowercaseName(self.prefix.0 + color_prefix_offset as u32),
            name: self.name.clone(),
        }
    }

    pub fn to_str(&self, needs_prefix: bool) -> Str {
        let Self { prefix, name } = self;
        if needs_prefix {
            format!("{prefix}{name}").into()
        } else {
            self.name.clone()
        }
    }
}

//! Constructor for [`super::ProductPuzzleBuilder`] using [`hypershape::Space`]
//! to cut polytope elements.

use std::collections::HashMap;
use std::sync::Arc;

use eyre::{Result, bail, ensure};
use hypergroup::IsometryGroup;
use hypermath::pga::Motor;
use hypermath::{Hyperplane, Point};
use hyperpuzzle_core::{Color, PerColor, PerPiece, PerSurface, Piece, Surface, TiVec};
use hypershape::PolytopeFate;
use itertools::Itertools;

use super::SurfaceBuilder;
use crate::geometry::PolytopeGeometry;
use crate::names::NameBiMap;

/// Constructor for [`super::ProductPuzzleBuilder`] using [`hypershape::Space`]
/// to cut polytope elements.
#[derive(Debug)]
pub(super) struct PuzzleBuilderFromHypershape {
    symmetry: IsometryGroup,
    generator_motors: Vec<Motor>,
    space: Arc<hypershape::Space>,
    pieces: PerPiece<PieceBuilder>,
    surfaces: PerSurface<SurfaceBuilder>,
    colors: PerColor<()>,
}

impl PuzzleBuilderFromHypershape {
    pub fn new(ndim: u8, symmetry: IsometryGroup) -> Result<Self> {
        let generator_motors = symmetry.generator_motors().into_values().cloned().collect();
        let space = hypershape::Space::new(ndim);
        let pieces = PerPiece::from_iter([PieceBuilder {
            polytope: space
                .add_primordial_cube(hypershape::PRIMORDIAL_CUBE_RADIUS)?
                .as_element()
                .id(),
            stickers: vec![],
        }]);

        Ok(Self {
            symmetry,
            generator_motors,
            space,
            pieces,
            surfaces: PerSurface::new(),
            colors: PerColor::new(),
        })
    }

    pub fn ndim(&self) -> u8 {
        self.space.ndim()
    }

    pub fn carve_symmetric(&mut self, plane: &Hyperplane) -> Result<()> {
        for plane in hypergroup::orbit_geometric(&self.generator_motors, plane.clone()) {
            let color = self.colors.push(())?;
            let cut = hypershape::Cut::carve(&self.space, plane);
            self.cut(cut, Some(color))?;
        }
        Ok(())
    }
    pub fn slice_symmetric(&mut self, plane: &Hyperplane) -> Result<()> {
        for plane in hypergroup::orbit_geometric(&self.generator_motors, plane.clone()) {
            let cut = hypershape::Cut::slice(&self.space, plane);
            self.cut(cut, None)?;
        }
        Ok(())
    }

    fn cut(&mut self, mut cut: hypershape::Cut, color: Option<Color>) -> Result<()> {
        let new_surface = if cut.params().inside == PolytopeFate::Remove
            || cut.params().outside == PolytopeFate::Remove
        {
            Some(self.surfaces.push(SurfaceBuilder {
                ndim: self.ndim(),
                centroid: Point::ORIGIN, // will be added later
                normal: cut.params().divider.normal().clone(),
            })?)
        } else {
            None
        };

        self.pieces = self
            .pieces
            .iter()
            .map(|(_, piece)| piece.cut(&mut cut, new_surface, color))
            .flatten_ok()
            .try_collect()?;
        if self.pieces.is_empty() {
            bail!("empty geometry");
        }
        Ok(())
    }

    /// Sets the centroid of each surface based on the stickers of one piece in
    /// the puzzle.
    ///
    /// Returns an error if there is not exactly one piece in the whole puzzle.
    pub fn set_surface_centroids_from_stickers_of_single_piece(
        &mut self,
        piece: Piece,
    ) -> Result<()> {
        ensure!(self.pieces.len() == 1, "expected exactly 1 piece");
        for sticker_data in &self.pieces[piece].stickers {
            let sticker_polytope = self.space.get(sticker_data.polytope);
            self.surfaces[sticker_data.surface].centroid = sticker_polytope.centroid()?.center();
        }
        Ok(())
    }

    pub fn into_product_puzzle_builder(self) -> Result<super::ProductPuzzleBuilder> {
        let ndim = self.ndim();

        let pieces = self.pieces.try_map_ref(|_, piece| {
            let facet_id_to_sticker: HashMap<hypershape::ElementId, &StickerBuilder> = piece
                .stickers
                .iter()
                .map(|sticker| (sticker.polytope, sticker))
                .collect();

            let piece_polytope = self.space.get(piece.polytope);
            eyre::Ok(super::PieceBuilder {
                polytope: PolytopeGeometry::from_polytope_element(piece_polytope)?,
                facets: piece_polytope
                    .boundary()
                    .map(|b| (b, facet_id_to_sticker.get(&b.id()).copied()))
                    .filter(|(_, sticker)| ndim <= 3 || sticker.is_some()) // remove internals in 4D+
                    .map(|(b, sticker)| {
                        eyre::Ok(super::PieceFacetBuilder {
                            polytope: PolytopeGeometry::from_polytope_element(b)?,
                            sticker_data: sticker.map(|sticker| super::StickerBuilder {
                                surface: sticker.surface,
                                color: sticker.color,
                            }),
                        })
                    })
                    .try_collect()?,
            })
        })?;

        Ok(super::ProductPuzzleBuilder {
            ndim,

            pieces,
            surfaces: self.surfaces,
            colors: self.colors,

            named_point_group_action: self.symmetry.action_on_points(&TiVec::new())?,
            named_point_names: NameBiMap::new(),

            axis_group_action: self.symmetry.action_on_points(&TiVec::new())?,
            axis_names: NameBiMap::new(),

            sticker_color_action: self.symmetry.action_on_points(&TiVec::new())?,
            sticker_color_names: NameBiMap::new(),

            isometry_group: self.symmetry.clone(),
        })
    }
}

#[derive(Debug)]
struct PieceBuilder {
    pub polytope: hypershape::ElementId,
    pub stickers: Vec<StickerBuilder>,
}

impl PieceBuilder {
    fn cut(
        &self,
        cut: &mut hypershape::Cut,
        new_surface: Option<Surface>,
        color: Option<Color>,
    ) -> Result<SimpleCutOutput<Self>> {
        let mut new_inside_stickers = vec![];
        let mut new_outside_stickers = vec![];

        // Cut stickers
        for sticker in &self.stickers {
            let output = sticker.cut(cut)?;
            new_inside_stickers.extend(output.inside);
            new_outside_stickers.extend(output.outside);
        }

        // Cut piece
        match cut.cut(self.polytope)? {
            hypershape::ElementCutOutput::Flush => bail!("piece is flush with cut"),
            hypershape::ElementCutOutput::NonFlush {
                inside,
                outside,
                intersection,
            } => {
                if let Some(polytope) = intersection
                    && let Some(surface) = new_surface
                    && let Some(color) = color
                {
                    new_inside_stickers.push(StickerBuilder {
                        polytope,
                        surface,
                        color,
                    });
                    new_outside_stickers.push(StickerBuilder {
                        polytope,
                        surface,
                        color,
                    });
                }

                Ok(SimpleCutOutput {
                    inside: inside.map(|polytope| {
                        let stickers = new_inside_stickers;
                        PieceBuilder { polytope, stickers }
                    }),
                    outside: outside.map(|polytope| {
                        let stickers = new_outside_stickers;
                        PieceBuilder { polytope, stickers }
                    }),
                })
            }
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct StickerBuilder {
    pub polytope: hypershape::ElementId,
    pub surface: Surface,
    pub color: Color,
}

impl StickerBuilder {
    fn cut(&self, cut: &mut hypershape::Cut) -> Result<SimpleCutOutput<Self>> {
        match cut.cut(self.polytope)? {
            hypershape::ElementCutOutput::Flush => Ok(SimpleCutOutput::EMPTY),
            hypershape::ElementCutOutput::NonFlush {
                inside, outside, ..
            } => Ok(SimpleCutOutput {
                inside: inside.map(|polytope| StickerBuilder { polytope, ..*self }),
                outside: outside.map(|polytope| StickerBuilder { polytope, ..*self }),
            }),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct SimpleCutOutput<T> {
    inside: Option<T>,
    outside: Option<T>,
}

impl<T> SimpleCutOutput<T> {
    const EMPTY: Self = Self {
        inside: None,
        outside: None,
    };
}

impl<T> IntoIterator for SimpleCutOutput<T> {
    type Item = T;

    type IntoIter =
        std::iter::FilterMap<std::array::IntoIter<Option<T>, 2>, fn(Option<T>) -> Option<T>>;

    fn into_iter(self) -> Self::IntoIter {
        [self.inside, self.outside]
            .into_iter()
            .filter_map(std::convert::identity)
    }
}

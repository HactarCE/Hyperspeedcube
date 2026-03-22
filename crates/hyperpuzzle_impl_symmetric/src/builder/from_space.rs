//! Constructor for [`ProductPuzzleBuilder`] using [`hypershape::Space`] to cut
//! polytope elements.

use std::collections::HashMap;
use std::sync::Arc;

use eyre::{OptionExt, Result, bail, ensure};
use hypergroup::IsometryGroup;
use hypermath::pga::Motor;
use hypermath::{Float, Hyperplane, Point, Vector};
use hyperpuzzle_core::{Axis, Color, PerAxis, PerColor, PerPiece, PerSurface, Piece, Surface};
use hypershape::PolytopeFate;
use hypuz_notation::Layer;
use itertools::Itertools;

use super::{PieceData, PieceFacetData, ProductPuzzleShape, StickerData, SurfaceData};
use crate::geometry::PolytopeGeometry;

/// Constructor for [`ProductPuzzleBuilder`] using [`hypershape::Space`]
/// to cut polytope elements.
#[derive(Debug)]
pub(super) struct PuzzleShapeBuilder {
    symmetry: IsometryGroup,
    /// Generating set for `symmetry`, cached for convenience.
    generator_motors: Vec<Motor>,

    space: Arc<hypershape::Space>,

    pieces: PerPiece<PieceShapeBuilder>,
    surfaces: PerSurface<SurfaceData>,
    colors: PerColor<()>,
}

impl PuzzleShapeBuilder {
    pub fn new(ndim: u8, symmetry: IsometryGroup, axis_count: usize) -> Result<Self> {
        let generator_motors = symmetry.generator_motors().into_values().cloned().collect();
        let space = hypershape::Space::new(ndim);
        let pieces = PerPiece::from_iter([PieceShapeBuilder {
            polytope: space
                .add_primordial_cube(hypershape::PRIMORDIAL_CUBE_RADIUS)?
                .as_element()
                .id(),
            stickers: vec![],
            grip_signature: PerAxis::new_with_len(axis_count),
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
            self.cut(cut, Some(color), None)?;
        }
        Ok(())
    }
    pub fn slice_symmetric<'a>(
        &mut self,
        axes: impl IntoIterator<Item = (Axis, &'a Vector)>,
        distance: Float,
        layer: Option<Layer>,
    ) -> Result<()> {
        for (axis, vector) in axes {
            let plane = Hyperplane::new(vector, distance).ok_or_eyre("bad cut plane")?;
            let cut = hypershape::Cut::slice(&self.space, plane);
            self.cut(cut, None, Some((axis, layer)))?;
        }
        Ok(())
    }

    fn cut(
        &mut self,
        mut cut: hypershape::Cut,
        color: Option<Color>,
        inside_grip: Option<(Axis, Option<Layer>)>,
    ) -> Result<()> {
        let new_surface = if cut.params().inside == PolytopeFate::Remove
            || cut.params().outside == PolytopeFate::Remove
        {
            Some(self.surfaces.push(SurfaceData {
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
            .map(|(_, piece)| piece.cut(&mut cut, new_surface, color, inside_grip))
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

    pub fn into_product_puzzle_shape(self) -> Result<ProductPuzzleShape> {
        let ndim = self.ndim();

        // Add internal facets, which are necessary when computing the direct
        // product of puzzles.
        let pieces = self.pieces.try_map_ref(|_, piece| {
            let facet_id_to_sticker: HashMap<hypershape::ElementId, &StickerShapeBuilder> = piece
                .stickers
                .iter()
                .map(|sticker| (sticker.polytope, sticker))
                .collect();

            let piece_polytope = self.space.get(piece.polytope);
            eyre::Ok(PieceData {
                polytope: PolytopeGeometry::from_polytope_element(piece_polytope)?,
                facets: piece_polytope
                    .boundary()
                    .map(|b| (b, facet_id_to_sticker.get(&b.id()).copied()))
                    .filter(|(_, sticker)| ndim <= 3 || sticker.is_some()) // remove internals in 4D+
                    .map(|(b, sticker)| {
                        eyre::Ok(PieceFacetData {
                            polytope: PolytopeGeometry::from_polytope_element(b)?,
                            sticker_data: sticker.map(|sticker| StickerData {
                                surface: sticker.surface,
                                color: sticker.color,
                            }),
                        })
                    })
                    .try_collect()?,
                grip_signature: piece.grip_signature.clone(),
            })
        })?;

        Ok(ProductPuzzleShape {
            symmetry: self.symmetry,
            pieces,
            surfaces: self.surfaces,
            colors: self.colors,
        })
    }
}

#[derive(Debug)]
struct PieceShapeBuilder {
    polytope: hypershape::ElementId,
    stickers: Vec<StickerShapeBuilder>,
    /// Grip signature, represented as a layer on each axis.
    ///
    /// This defaults to `None`, which indicates that the piece does not move
    /// with any layer on the axis.
    grip_signature: PerAxis<Option<Layer>>,
}

impl PieceShapeBuilder {
    fn cut(
        &self,
        cut: &mut hypershape::Cut,
        new_surface: Option<Surface>,
        color: Option<Color>,
        inside_grip: Option<(Axis, Option<Layer>)>,
    ) -> Result<SimpleCutOutput<Self>> {
        let mut inside_stickers = vec![];
        let mut outside_stickers = vec![];

        // Cut stickers
        let mut flush_sticker = None;
        for &sticker in &self.stickers {
            match cut.cut(sticker.polytope)? {
                hypershape::ElementCutOutput::Flush => flush_sticker = Some(sticker),
                hypershape::ElementCutOutput::NonFlush {
                    inside, outside, ..
                } => {
                    inside_stickers.extend(inside.map(|polytope| StickerShapeBuilder {
                        polytope,
                        ..sticker
                    }));
                    outside_stickers.extend(outside.map(|polytope| StickerShapeBuilder {
                        polytope,
                        ..sticker
                    }));
                }
            };
        }

        // Cut piece
        match cut.cut(self.polytope)? {
            hypershape::ElementCutOutput::Flush => bail!("piece is flush with cut"),
            hypershape::ElementCutOutput::NonFlush {
                inside,
                outside,
                intersection,
            } => {
                if let Some(flush_sticker) = flush_sticker {
                    inside_stickers.push(flush_sticker);
                    outside_stickers.push(flush_sticker);
                } else if let Some(polytope) = intersection
                    && let Some(surface) = new_surface
                    && let Some(color) = color
                {
                    inside_stickers.push(StickerShapeBuilder {
                        polytope,
                        surface,
                        color,
                    });
                    outside_stickers.push(StickerShapeBuilder {
                        polytope,
                        surface,
                        color,
                    });
                }

                let mut inside_grip_signature = self.grip_signature.clone();
                let outside_grip_signature = self.grip_signature.clone();
                if let Some((axis, layer)) = inside_grip {
                    inside_grip_signature[axis] = layer;
                }

                Ok(SimpleCutOutput {
                    inside: inside.map(|polytope| PieceShapeBuilder {
                        polytope,
                        stickers: inside_stickers,
                        grip_signature: inside_grip_signature,
                    }),
                    outside: outside.map(|polytope| PieceShapeBuilder {
                        polytope,
                        stickers: outside_stickers,
                        grip_signature: outside_grip_signature,
                    }),
                })
            }
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct StickerShapeBuilder {
    polytope: hypershape::ElementId,
    surface: Surface,
    color: Color,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct SimpleCutOutput<T> {
    inside: Option<T>,
    outside: Option<T>,
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

//! Constructor for [`ProductPuzzleBuilder`] using [`hypershape::Space`] to cut
//! polytope elements.

use eyre::{OptionExt, Result, bail, ensure};
use hypergroup::{CoxeterMatrix, IsometryGroup};
use hypermath::{APPROX, ApproxHashMap, Centroid, Hyperplane, Point};
use hyperpuzzle_core::{
    Color, IndexOverflow, PerAxis, PerColor, PerPiece, PerSurface, Piece, Surface,
};
use itertools::Itertools;

use super::{PieceData, PieceFacetData, ProductPuzzleShape, StickerData, SurfaceData};
use crate::geometry::PolytopeGeometry;

/// Constructor for [`super::ProductPuzzleBuilder`] using [`hypershape::Space`]
/// to cut polytope elements.
///
/// This type cannot be direct-producted.
#[derive(Debug)]
pub(super) struct PuzzleShapeFactorBuilder {
    group: IsometryGroup,

    space: hypershape::Space,

    pieces: PerPiece<PieceShapeBuilder>,
    surfaces: PerSurface<SurfaceData>,
    color_names: PerColor<String>,

    hyperplane_to_surface: ApproxHashMap<Hyperplane, Surface>,
}

impl PuzzleShapeFactorBuilder {
    pub fn new(coxeter_matrix: CoxeterMatrix, group: IsometryGroup) -> Result<Self> {
        let mut space = hypershape::Space::new(group.ndim())?;
        let mut initial_piece = space.primordial_cube().into();
        for mirror_vector in coxeter_matrix.mirrors()?.cols() {
            let mirror_plane =
                Hyperplane::new(mirror_vector, 0.0).ok_or_eyre("bad mirror vector")?;
            initial_piece = hypershape::Cut::carve_portal(mirror_plane)
                .cut(&mut space, initial_piece)?
                .outside()
                .ok_or_eyre("fundamental region is empty")?;
        }
        let pieces = PerPiece::from_iter([PieceShapeBuilder {
            polytope: initial_piece,
            stickers: vec![],
        }]);

        Ok(Self {
            group,

            space,

            pieces,
            surfaces: PerSurface::new(),
            color_names: PerColor::new(),

            hyperplane_to_surface: ApproxHashMap::new(APPROX),
        })
    }

    pub fn ndim(&self) -> u8 {
        self.space.ndim()
    }

    pub fn surface_count(&self) -> usize {
        self.surfaces.len()
    }

    pub fn add_color(&mut self, name: String) -> Result<Color, IndexOverflow> {
        self.color_names.push(name)
    }

    pub fn carve(&mut self, plane: Hyperplane, color: Color) -> Result<()> {
        let new_surface = self.surfaces.push(SurfaceData {
            centroid: Point::ORIGIN, // will be computed later
            hyperplane: plane.clone(),
            color,
        })?;
        let old_surface = self
            .hyperplane_to_surface
            .insert(plane.clone(), new_surface);
        if old_surface.is_some() {
            bail!("duplicate surfaces");
        }
        let cut = hypershape::Cut::carve(plane);
        self.cut(cut)?;
        Ok(())
    }
    pub fn slice(&mut self, plane: Hyperplane) -> Result<()> {
        let cut = hypershape::Cut::slice(plane);
        self.cut(cut)?;
        Ok(())
    }

    fn cut(&mut self, mut cut: hypershape::Cut) -> Result<()> {
        self.pieces = self
            .pieces
            .iter()
            .map(|(_, piece)| piece.cut(&mut self.space, &mut cut))
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

        let mut centroids = PerSurface::<Centroid>::new_with_len(self.surface_count());

        for sticker_data in &self.pieces[piece].stickers {
            let sticker_polytope = self.space.get(sticker_data.polytope);

            let centroid = sticker_polytope.centroid()?;
            let center = centroid.center();
            let weight = centroid.weight();

            // Expand each centroid by symmetry.
            // This could be optimized by first projecting the centroid to the
            // mirror planes touched by sticker polytope and then orbiting
            // *that* centroid.
            for (h, c) in hypergroup::orbit_geometric(
                self.group.generator_motors(),
                (sticker_polytope.as_facet()?.hyperplane()?, center),
            ) {
                let s = *self
                    .hyperplane_to_surface
                    .get(h)
                    .ok_or_eyre("missing surface")?;
                centroids[s] += Centroid::new(&c, weight);
            }
        }

        for (s, centroid) in centroids {
            self.surfaces[s].centroid = centroid.center();
        }

        Ok(())
    }

    pub fn into_product_puzzle_shape(mut self) -> Result<ProductPuzzleShape> {
        let ndim = self.ndim();

        let pieces: PerPiece<PieceData> = self
            .pieces
            .iter_values()
            .map(|piece| {
                let unfolded = self.space.unfold(piece.polytope)?;
                let piece_polytope = self.space.get(unfolded);
                let sticker_facet_id_list: Vec<hypershape::FacetId> = piece_polytope
                    .boundary()
                    .map(|b| b.as_facet())
                    .filter_ok(|f| {
                        let Ok(h) = f.hyperplane() else { return false };
                        self.hyperplane_to_surface.contains_key(h)
                    })
                    .map_ok(|f| f.id())
                    .try_collect()?;
                let sticker_shrink_vectors = piece_polytope
                    .as_polytope()?
                    .sticker_shrink_vectors(&sticker_facet_id_list)?;
                let init_piece_data = PieceData {
                    polytope: PolytopeGeometry::from_polytope_element(
                        piece_polytope,
                        &sticker_shrink_vectors,
                    )?,
                    facets: piece_polytope
                        .boundary()
                        .map(|b| {
                            eyre::Ok((
                                b,
                                self.hyperplane_to_surface.get(b.as_facet()?.hyperplane()?),
                            ))
                        })
                        .filter_ok(|(_, opt_surface)| ndim <= 3 || opt_surface.is_some()) // remove internals in 4D+
                        .map(|result| {
                            let (b, opt_surface) = result?;
                            eyre::Ok(PieceFacetData {
                                polytope: PolytopeGeometry::from_polytope_element(
                                    b,
                                    &sticker_shrink_vectors,
                                )?,
                                sticker_data: opt_surface.map(|&surface| StickerData { surface }),
                            })
                        })
                        .try_collect()?,
                    grip_signature: PerAxis::new(), // will be computed later
                };

                let mut centroids_seen = ApproxHashMap::<Centroid, ()>::new(APPROX);
                centroids_seen.insert(init_piece_data.polytope.centroid.clone(), ());

                eyre::Ok(hypergroup::orbit_collect(
                    init_piece_data,
                    self.group.generator_motors(),
                    |_, piece_data, g| {
                        let new_centroid = g.transform(&piece_data.polytope.centroid);

                        centroids_seen
                            .insert(new_centroid.clone(), ())
                            .is_none()
                            .then(|| {
                                let polytope = g.transform(&piece_data.polytope);
                                let facets = piece_data
                                    .facets
                                    .iter()
                                    .map(|f| PieceFacetData {
                                        polytope: g.transform(&f.polytope),
                                        sticker_data: f.sticker_data.as_ref().and_then(|s| {
                                            let h =
                                                g.transform(&self.surfaces[s.surface].hyperplane);
                                            Some(StickerData {
                                                surface: *self.hyperplane_to_surface.get(h)?,
                                            })
                                        }),
                                    })
                                    .collect();
                                PieceData {
                                    polytope,
                                    facets,
                                    grip_signature: PerAxis::new(), // will be computed later
                                }
                            })
                    },
                ))
            })
            .flatten_ok()
            .try_collect()?;

        Ok(ProductPuzzleShape {
            group: self.group,
            colors: self.color_names.map(|_, name| (0, name)),
            pieces,
            surfaces: self.surfaces,
        })
    }
}

#[derive(Debug)]
struct PieceShapeBuilder {
    polytope: hypershape::ElementId,
    stickers: Vec<StickerShapeBuilder>,
}

impl PieceShapeBuilder {
    fn cut(
        &self,
        space: &mut hypershape::Space,
        cut: &mut hypershape::Cut,
    ) -> Result<SimpleCutOutput<Self>> {
        let mut inside_stickers = vec![];
        let mut outside_stickers = vec![];

        // Cut stickers
        let mut flush_sticker = None;
        for &sticker in &self.stickers {
            match cut.cut(space, sticker.polytope)? {
                hypershape::ElementCutOutput::Flush => flush_sticker = Some(sticker),
                hypershape::ElementCutOutput::NonFlush {
                    inside, outside, ..
                } => {
                    inside_stickers.extend(inside.map(|polytope| StickerShapeBuilder { polytope }));
                    outside_stickers
                        .extend(outside.map(|polytope| StickerShapeBuilder { polytope }));
                }
            };
        }

        // Cut piece
        match cut.cut(space, self.polytope)? {
            hypershape::ElementCutOutput::Flush => bail!("piece is flush with cut"),
            hypershape::ElementCutOutput::NonFlush {
                inside,
                outside,
                intersection,
            } => {
                if let Some(flush_sticker) = flush_sticker {
                    inside_stickers.push(flush_sticker);
                    outside_stickers.push(flush_sticker);
                } else if let Some(polytope) = intersection {
                    inside_stickers.push(StickerShapeBuilder { polytope });
                    outside_stickers.push(StickerShapeBuilder { polytope });
                }

                Ok(SimpleCutOutput {
                    inside: inside.map(|polytope| PieceShapeBuilder {
                        polytope,
                        stickers: inside_stickers,
                    }),
                    outside: outside.map(|polytope| PieceShapeBuilder {
                        polytope,
                        stickers: outside_stickers,
                    }),
                })
            }
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct StickerShapeBuilder {
    polytope: hypershape::ElementId,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct SimpleCutOutput<T> {
    inside: Option<T>,
    outside: Option<T>,
}

impl<T> IntoIterator for SimpleCutOutput<T> {
    type Item = T;

    type IntoIter = std::iter::Flatten<std::array::IntoIter<Option<T>, 2>>;

    fn into_iter(self) -> Self::IntoIter {
        [self.inside, self.outside].into_iter().flatten()
    }
}

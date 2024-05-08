use std::sync::{Arc, Weak};

use eyre::{bail, Context, Result};
use hypermath::{Hyperplane, VecMap};
use hypershape::prelude::*;
use itertools::Itertools;
use parking_lot::Mutex;

use super::{ColorSystemBuilder, PieceBuilder};
use crate::puzzle::*;

/// Soup of shapes being constructed.
#[derive(Debug)]
pub struct ShapeBuilder {
    /// Reference-counted pointer to this struct.
    pub this: Weak<Mutex<Self>>,

    /// Shape ID.
    pub id: Option<String>,

    /// Space where the puzzle exists.
    pub space: Arc<Mutex<Space>>,

    /// Symmetry group of the whole shape.
    pub symmetry: Option<SchlafliSymbol>,

    /// Puzzle pieces.
    pub pieces: PerPiece<PieceBuilder>,
    /// Pieces that are not defunct (removed or cut) and so should be included
    /// in the final puzzle.
    pub active_pieces: PieceSet,

    /// Facet colors.
    pub colors: ColorSystemBuilder,
}
impl ShapeBuilder {
    /// Constructs a shape builder that starts with an empty Euclidean space.
    pub fn new_empty(id: Option<String>, space: Arc<Mutex<Space>>) -> Result<Arc<Mutex<Self>>> {
        Ok(Arc::new_cyclic(|this| {
            Mutex::new(Self {
                this: this.clone(),

                id,

                space,

                symmetry: None,

                pieces: PerPiece::new(),
                active_pieces: PieceSet::new(),

                colors: ColorSystemBuilder::new(),
            })
        }))
    }

    /// Constructs a shape builder that starts with a single solid piece (the
    /// primordial cube)
    pub fn new_with_primordial_cube(
        id: Option<String>,
        space: Arc<Mutex<Space>>,
    ) -> Result<Arc<Mutex<Self>>> {
        let this = Self::new_empty(id, Arc::clone(&space))?;
        let mut this_guard = this.lock();
        let primordial_cube = space
            .lock()
            .add_primordial_cube(crate::PRIMORDIAL_CUBE_RADIUS)?;
        let root_piece_builder = PieceBuilder::new(primordial_cube, VecMap::new());
        let root_piece = this_guard.pieces.push(root_piece_builder)?;
        this_guard.active_pieces.insert(root_piece);
        drop(this_guard);
        Ok(this)
    }

    /// Returns an `Arc` reference to the shape builder.
    pub fn arc(&self) -> Arc<Mutex<Self>> {
        self.this
            .upgrade()
            .expect("`ShapeBuilder` removed from `Arc`")
    }

    /// Returns the number of dimensions of the underlying space.
    pub fn ndim(&self) -> u8 {
        self.space.lock().ndim()
    }

    /// Returns a deep copy of the shape. This is a relatively expensive
    /// operation.
    pub fn clone(&self, space: &Arc<Mutex<Space>>) -> Result<Arc<Mutex<Self>>> {
        let old_space = self.space.lock();
        let mut new_space = space.lock();
        let mut map = SpaceMap::new(&old_space, &mut new_space)?;

        let pieces: PerPiece<PieceBuilder> = self
            .active_pieces
            .iter()
            .map(|piece| {
                let polytope = self.pieces[piece].polytope;
                let stickers = self.pieces[piece]
                    .stickers
                    .iter()
                    .map(|(&k, &v)| eyre::Ok((map.map(k)?, v)))
                    .try_collect()?;
                eyre::Ok(PieceBuilder {
                    polytope: map.map(polytope)?,
                    cut_result: PieceSet::new(),
                    stickers,
                })
            })
            .try_collect()?;
        let active_pieces = pieces.iter_keys().collect();

        Ok(Arc::new_cyclic(|this| {
            Mutex::new(Self {
                this: this.clone(),

                id: self.id.clone(),

                space: Arc::clone(space),

                symmetry: self.symmetry.clone(),
                pieces,
                active_pieces,
                colors: self.colors.clone(),
            })
        }))
    }

    /// Cuts each piece by a cut, throwing away the portions that are outside
    /// the cut. Each piece in the old set becomes defunct, and each piece in
    /// the new set inherits its active status from the corresponding piece in
    /// the old set.
    ///
    /// If `pieces` is `None`, then it is assumed to be all active pieces.
    pub fn carve(
        &mut self,
        pieces: Option<&PieceSet>,
        cut_plane: Hyperplane,
        inside_color: Option<Color>,
    ) -> Result<()> {
        let mut cut = Cut::carve(cut_plane);
        self.cut_and_deactivate_pieces(&mut cut, pieces, inside_color, None)
    }
    /// Cuts each piece by a cut, keeping all results. Each piece in the old set
    /// becomes defunct, and each piece in the new set inherits its active
    /// status from the corresponding piece in the old set.
    ///
    /// If `pieces` is `None`, then it is assumed to be all active pieces.
    pub fn slice(
        &mut self,
        pieces: Option<&PieceSet>,
        cut_plane: Hyperplane,
        inside_color: Option<Color>,
        outside_color: Option<Color>,
    ) -> Result<()> {
        let mut cut = Cut::slice(cut_plane);
        self.cut_and_deactivate_pieces(&mut cut, pieces, inside_color, outside_color)
    }
    fn cut_and_deactivate_pieces(
        &mut self,
        cut: &mut Cut,
        pieces: Option<&PieceSet>,
        inside_color: Option<Color>,
        outside_color: Option<Color>,
    ) -> Result<()> {
        let pieces = match pieces {
            Some(piece_set) => self.update_piece_set(piece_set),
            None => self.active_pieces.clone(),
        };

        let mut space = self.space.lock();

        for old_piece in pieces.iter() {
            let inside_polytope;
            let outside_polytope;
            let mut inside_stickers = VecMap::new();
            let mut outside_stickers = VecMap::new();

            // Cut the old piece and add the new pieces as active.
            let old_piece_polytope = self.pieces[old_piece].polytope;
            match space
                .cut_polytope(old_piece_polytope, cut)
                .context("error cutting piece")?
            {
                PolytopeCutOutput::Flush => bail!("piece is flush with cut"),

                out @ PolytopeCutOutput::NonFlush {
                    inside,
                    outside,
                    intersection,
                } => {
                    if intersection.is_none() && out.is_unchanged_from(old_piece_polytope) {
                        // Leave this piece unchanged.
                        continue;
                    }

                    inside_polytope = inside;
                    outside_polytope = outside;

                    if let Some(p) = intersection {
                        if let Some(c) = inside_color {
                            inside_stickers.insert(p, c);
                        }
                        if let Some(c) = outside_color {
                            outside_stickers.insert(p, c);
                        }
                    }
                }
            }

            // Cut the old stickers.
            for (&old_sticker_polytope, &old_color) in &self.pieces[old_piece].stickers {
                match space
                    .cut_polytope(old_sticker_polytope, cut)
                    .context("error cutting sticker")?
                {
                    PolytopeCutOutput::Flush => (), // Leave the sticker unchanged
                    PolytopeCutOutput::NonFlush {
                        inside, outside, ..
                    } => {
                        // Use `get_or_insert()` instead to keep old color for
                        // flush stickers instead of assigning the new color.
                        if let Some(p) = inside {
                            inside_stickers.insert(p, old_color);
                        }
                        if let Some(p) = outside {
                            outside_stickers.insert(p, old_color);
                        }
                    }
                }
            }

            let new_inside_piece = match inside_polytope {
                Some(p) => Some(self.pieces.push(PieceBuilder::new(p, inside_stickers))?),
                None => None,
            };
            let new_outside_piece = match outside_polytope {
                Some(p) => Some(self.pieces.push(PieceBuilder::new(p, outside_stickers))?),
                None => None,
            };

            self.active_pieces.extend(new_inside_piece);
            self.active_pieces.extend(new_outside_piece);

            // The old piece is defunct, so deactivate it and record its cut
            // result.
            self.active_pieces.remove(&old_piece);
            self.pieces[old_piece].cut_result =
                itertools::chain(new_inside_piece, new_outside_piece).collect();

            self.active_pieces.remove(&old_piece);
        }

        Ok(())
    }

    /// Updates a piece set, replacing defunct pieces with their cut results.
    /// Call this before doing anything with a piece set to prevent operating on
    /// defunct pieces.
    pub fn update_piece_set(&self, piece_set: &PieceSet) -> PieceSet {
        let mut queue = piece_set.iter().collect_vec();
        let mut output = PieceSet::new();
        while let Some(old_piece) = queue.pop() {
            if self.active_pieces.contains(&old_piece) {
                output.insert(old_piece);
            } else {
                queue.extend(self.pieces[old_piece].cut_result.iter());
            }
        }
        output
    }
}

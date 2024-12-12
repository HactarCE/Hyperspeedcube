use std::sync::Arc;

use hypermath::prelude::*;
use itertools::Itertools;

use crate::{Axis, LayerMask, LayeredTwist, PerPiece, Piece, PieceMask, Puzzle};

/// Instance of a puzzle with a particular state.
#[derive(Debug, Clone)]
pub struct PuzzleState {
    /// Immutable puzzle type info.
    puzzle_type: Arc<Puzzle>,
    /// Position and rotation of each piece.
    piece_transforms: PerPiece<pga::Motor>,
}
impl PuzzleState {
    /// Constructs a new instance of a puzzle.
    pub fn new(puzzle_type: Arc<Puzzle>) -> Self {
        let ident = pga::Motor::ident(puzzle_type.ndim());
        let piece_transforms = puzzle_type.pieces.map_ref(|_, _| ident.clone());
        PuzzleState {
            puzzle_type,
            piece_transforms,
        }
    }
    /// Returns the puzzle type
    pub fn ty(&self) -> &Arc<Puzzle> {
        &self.puzzle_type
    }
    /// Returns the position and rotation of each piece.
    pub fn piece_transforms(&self) -> &PerPiece<pga::Motor> {
        &self.piece_transforms
    }
    /// Returns the position and rotation of each piece during an arbitrary
    /// animation affecting a subset of pieces.
    pub fn partial_piece_transforms(
        &self,
        grip: &PieceMask,
        transform: &pga::Motor,
    ) -> PerPiece<pga::Motor> {
        self.piece_transforms()
            .map_ref(|piece, static_transform| match grip.contains(piece) {
                true => transform * static_transform,
                _ => static_transform.clone(),
            })
    }

    /// Does a twist, or returns an error containing the set of pieces that
    /// prevented the twist.
    pub fn do_twist(&self, twist: LayeredTwist) -> Result<Self, Vec<Piece>> {
        let twist_info = &self.puzzle_type.twists[twist.transform];
        let grip = self.compute_grip(twist_info.axis, twist.layers);

        // Check for split pieces, which prevent the turn.
        let split_pieces = grip
            .iter_filter(|_piece, &which_side| which_side == WhichSide::Split)
            .collect_vec();
        if !split_pieces.is_empty() {
            return Err(split_pieces);
        }

        let piece_transforms = self.piece_transforms.map_ref(|piece, piece_transform| {
            if grip[piece] == WhichSide::Inside {
                &twist_info.transform * &self.piece_transforms[piece]
            } else {
                piece_transform.clone()
            }
        });

        Ok(Self {
            puzzle_type: Arc::clone(&self.puzzle_type),
            piece_transforms,
        })
    }

    /// Returns the set of pieces on the inside of a grip (axis + layer mask).
    /// This considers blocking pieces to be outside the grip; use
    /// `compute_grip()` to see which pieces are blocking a twist.
    pub fn compute_gripped_pieces(&self, axis: Axis, layers: LayerMask) -> PieceMask {
        PieceMask::from_iter(
            self.puzzle_type.pieces.len(),
            self.compute_grip(axis, layers)
                .iter_filter(|_, &status| status == WhichSide::Inside),
        )
    }

    /// Returns each piece's location with respect to a grip (axis + layer
    /// mask). A piece may be inside the grip, outside the grip, or blocking the
    /// grip. [`WhichSide::Flush`] is not used.
    pub fn compute_grip(&self, axis: Axis, layers: LayerMask) -> PerPiece<WhichSide> {
        let Ok(axis) = self.puzzle_type.axes.get(axis) else {
            log::error!("bad axis ID");
            return self.puzzle_type.pieces.map_ref(|_, _| WhichSide::Split);
        };

        let grip_layers = layers
            .iter()
            .filter_map(|layer| axis.layers.get(layer).ok())
            .collect_vec();

        let mut segments: Vec<(Hyperplane, Option<Hyperplane>)> = vec![];
        for layer in grip_layers {
            if let Some((_, last_top)) = segments.last_mut() {
                if *last_top == Some(layer.bottom.flip()) {
                    *last_top = layer.top.clone();
                    continue;
                }
            }
            segments.push((layer.bottom.clone(), layer.top.clone()));
        }

        let space = &self.puzzle_type.space;

        self.puzzle_type.pieces.map_ref(|piece, piece_info| {
            // IIFE to mimic try_block
            (|| {
                let polytope = piece_info.polytope;
                let piece_transform = &self.piece_transforms[piece];
                let rev_piece_transform = piece_transform.reverse();
                let mut is_inside_any = false;
                'per_segment: for (bottom, top) in &segments {
                    for cut in [Some(bottom), top.as_ref()] {
                        let Some(cut) = cut else { continue };

                        let transformed_cut = rev_piece_transform.transform(cut);

                        match space.get(polytope).is_on_which_side_of(&transformed_cut) {
                            WhichSide::Outside => continue 'per_segment, // not in this segment; continue to next segment
                            WhichSide::Split => return Ok(WhichSide::Split), // split by one segment; cannot turn!
                            _ => (),
                        }
                    }
                    // This piece wasn't excluded by either the bottom or the
                    // top, so it should be good!
                    is_inside_any = true;
                }
                match is_inside_any {
                    true => Ok(WhichSide::Inside),
                    false => Ok(WhichSide::Outside),
                }
            })()
            .unwrap_or_else(|e: eyre::Report| {
                log::error!("{e}");
                WhichSide::Split
            })
        })
    }

    /// Returns the smallest layer mask on `axis` that contains `piece`.
    pub fn min_layer_mask(&self, axis: Axis, piece: Piece) -> Option<LayerMask> {
        let space = &self.puzzle_type.space;

        let piece_transform = &self.piece_transforms[piece];

        // TODO: This assumes the piece only spans one layer. It does not
        //       account for bandaging.
        self.ty().axes[axis]
            .layers
            .find(|_layer, layer_info| {
                space
                    .get(self.ty().pieces[piece].polytope)
                    .vertex_set()
                    .all(|v| {
                        let p = piece_transform.transform_point(v.pos());
                        layer_info.bottom.location_of_point(&p) != PointWhichSide::Outside && {
                            match &layer_info.top {
                                Some(top) => top.location_of_point(&p) != PointWhichSide::Outside,
                                None => true,
                            }
                        }
                    })
            })
            .map(LayerMask::from)
    }
}

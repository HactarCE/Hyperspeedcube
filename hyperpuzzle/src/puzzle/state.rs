use std::sync::Arc;

use hypermath::prelude::*;
use hypershape::prelude::*;
use itertools::Itertools;
use parking_lot::MutexGuard;

use crate::{Axis, LayerMask, PerPiece, Piece, Puzzle, Twist};

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
    /// Returns the position and rotation of each piece during an animation.
    ///
    /// `t` ranges from `0.0` to `1.0`.
    pub fn animated_piece_transforms(
        &self,
        twist: Twist,
        layers: LayerMask,
        t: Float,
    ) -> PerPiece<pga::Motor> {
        let grip = self.compute_grip(self.ty().twists[twist].axis, layers);
        let twist_transform = self.ty().partial_twist_transform(twist, t);
        self.piece_transforms()
            .map_ref(|piece, current_piece_transform| match grip[piece] {
                WhichSide::Inside => &twist_transform * current_piece_transform,
                _ => current_piece_transform.clone(),
            })
    }

    fn space(&self) -> MutexGuard<'_, Space> {
        self.puzzle_type.space.lock()
    }

    /// Does a twist, or returns an error containing the set of pieces that
    /// prevented the twist.
    #[must_use]
    pub fn do_twist(&self, twist: Twist, layers: LayerMask) -> Result<Self, Vec<Piece>> {
        let twist = &self.puzzle_type.twists[twist];
        let grip = self.compute_grip(twist.axis, layers);

        // Check for split pieces, which prevent the turn.
        let split_pieces = grip
            .iter_filter(|_piece, &which_side| which_side == WhichSide::Split)
            .collect_vec();
        if !split_pieces.is_empty() {
            return Err(split_pieces);
        }

        let piece_transforms = self.piece_transforms.map_ref(|piece, piece_transform| {
            if grip[piece] == WhichSide::Inside {
                &twist.transform * &self.piece_transforms[piece]
            } else {
                piece_transform.clone()
            }
        });

        Ok(Self {
            puzzle_type: Arc::clone(&self.puzzle_type),
            piece_transforms,
        })
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

        let mut segments = vec![];
        for layer in grip_layers {
            if let Some((_, last_top)) = segments.last_mut() {
                if *last_top == Some(layer.bottom.flip()) {
                    *last_top = layer.top.clone();
                    continue;
                }
            }
            segments.push((layer.bottom.clone(), layer.top.clone()));
        }

        let mut space = self.space();

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

                        match space.which_side_has_polytope(&transformed_cut, polytope) {
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
}

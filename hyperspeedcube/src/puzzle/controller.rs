use std::sync::Arc;

use hypermath::pga::Motor;
use hyperpuzzle::{LayerMask, PerPiece, Puzzle, PuzzleState, Twist};
use instant::Instant;

use super::animations::{BlockingPiecesAnimationState, TwistAnimation, TwistAnimationState};
use crate::preferences::Preferences;

#[derive(Debug, Clone)]
pub struct PuzzleController {
    /// Latest puzzle state, not including any transient rotation.
    puzzle: PuzzleState,

    /// Twist animation state.
    twist_anim: TwistAnimationState,
    /// Blocking pieces animation state.
    blocking_pieces: BlockingPiecesAnimationState,
    /// Time of last frame, or `None` if we are not in the middle of an animation.
    last_frame_time: Option<Instant>,

    /// Latest visual piece transforms.
    cached_piece_transforms: PerPiece<Motor>,
}
impl PuzzleController {
    pub fn new(puzzle: &Arc<Puzzle>) -> Self {
        let puzzle = PuzzleState::new(Arc::clone(puzzle));
        let cached_piece_transforms = puzzle.piece_transforms().clone();
        Self {
            puzzle,

            twist_anim: TwistAnimationState::default(),
            blocking_pieces: BlockingPiecesAnimationState::default(),
            last_frame_time: None,

            cached_piece_transforms,
        }
    }

    pub fn puzzle(&self) -> &PuzzleState {
        &self.puzzle
    }
    pub fn puzzle_type(&self) -> &Arc<Puzzle> {
        self.puzzle.ty()
    }

    pub fn piece_transforms(&self) -> &PerPiece<Motor> {
        &self.cached_piece_transforms
    }
    fn update_piece_transforms(&mut self) {
        self.cached_piece_transforms = match self.twist_anim.current() {
            Some((anim, t)) => {
                anim.state
                    .animated_piece_transforms(anim.twist, anim.layers, t as _)
            }
            None => self.puzzle.piece_transforms().clone(),
        };
    }

    pub fn do_twist(&mut self, twist: Twist, layers: LayerMask) {
        match self.puzzle.do_twist(twist, layers) {
            Ok(new_state) => {
                let old_state = std::mem::replace(&mut self.puzzle, new_state);
                self.twist_anim.push(TwistAnimation {
                    state: old_state,
                    twist,
                    layers,
                });
                self.blocking_pieces.clear();
            }
            Err(blocking_pieces) => self.blocking_pieces.set(blocking_pieces),
        }
    }

    /// Advances the puzzle geometry and internal state to the next frame, using
    /// the given time delta between this frame and the last. Returns whether
    /// the puzzle must be redrawn.
    pub fn update_geometry(&mut self, prefs: &Preferences) -> bool {
        let now = Instant::now();
        let delta = match self.last_frame_time {
            Some(then) => now - then,
            None => prefs.gfx.frame_duration(),
        };

        let mut needs_redraw = false;

        // TODO: animate view angle offset
        // // Animate view angle offset.
        // if !self.view_angle.is_frozen {
        //     let offset = &mut self.view_angle.current;

        //     let decay_multiplier = VIEW_ANGLE_OFFSET_DECAY_RATE.powf(delta.as_secs_f32());
        //     let new_offset = Quaternion::one().slerp(*offset, decay_multiplier);
        //     if offset.s == new_offset.s {
        //         // Stop the animation once we're not making any more progress.
        //         *offset = Quaternion::one();
        //     } else {
        //         *offset = new_offset;
        //     }
        // }

        if self.twist_anim.proceed(delta, &prefs.interaction) {
            self.update_piece_transforms();
            needs_redraw = true;
        }
        needs_redraw |= self.blocking_pieces.proceed(&prefs.interaction);

        if needs_redraw {
            self.last_frame_time = Some(now);
        } else {
            self.last_frame_time = None;
        }

        needs_redraw
    }

    pub fn blocking_pieces(&self) -> &BlockingPiecesAnimationState {
        &self.blocking_pieces
    }
}

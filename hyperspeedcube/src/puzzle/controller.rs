use std::sync::Arc;

use float_ord::FloatOrd;
use hypermath::pga::{Axes, Motor};
use hyperpuzzle::{Axis, LayerMask, PerPiece, Puzzle, PuzzleState, Twist};
use instant::Instant;

use super::animations::{BlockingPiecesAnimationState, TwistAnimation, TwistAnimationState};
use crate::preferences::Preferences;

/// Puzzle simulation, which manages the puzzle state, animations, undo stack,
/// etc.
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

    partial_twist_drag_state: Option<(Axis, LayerMask, Motor)>,

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

            partial_twist_drag_state: None,

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
        self.cached_piece_transforms = None
            .or_else(|| {
                self.twist_anim.current().map(|(anim, t)| {
                    anim.state.animated_piece_transforms(
                        &anim.initial_transform,
                        anim.twist,
                        anim.layers,
                        t as _,
                    )
                })
            })
            .or_else(|| {
                self.partial_twist_drag_state
                    .as_ref()
                    .map(|(axis, layers, motor)| {
                        let grip = self.puzzle.compute_grip(*axis, *layers);
                        self.puzzle.partial_piece_transforms(grip, motor)
                    })
            })
            .unwrap_or_else(|| self.puzzle.piece_transforms().clone());
    }

    pub fn do_twist(&mut self, twist: Twist, layers: LayerMask) {
        match self.puzzle.do_twist(twist, layers) {
            Ok(new_state) => {
                let old_state = std::mem::replace(&mut self.puzzle, new_state);
                self.twist_anim.push(TwistAnimation {
                    state: old_state,
                    twist,
                    layers,
                    initial_transform: match self.partial_twist_drag_state.take() {
                        Some((_, _, m)) => m,
                        None => Motor::ident(self.puzzle.ty().ndim()),
                    },
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

    pub fn set_partial_twist_drag_state(&mut self, value: Option<(Axis, LayerMask, Motor)>) {
        if value.is_none() {
            if let Some((axis, layers, m)) = &self.partial_twist_drag_state {
                if let Some((twist, _)) = self
                    .puzzle_type()
                    .twists
                    .iter()
                    .filter(|(_, twist_info)| twist_info.axis == *axis)
                    .max_by_key(|(_, twist_info)| {
                        FloatOrd(Motor::dot(&twist_info.transform, &m).abs())
                    })
                    .filter(|(_, twist_info)| {
                        Motor::dot(&twist_info.transform, &m).abs() > m.get(Axes::SCALAR)
                    })
                {
                    self.do_twist(twist, *layers);
                }
            }
        }
        self.partial_twist_drag_state = value;
        self.update_piece_transforms(); // TODO: is this necessary?
    }
}

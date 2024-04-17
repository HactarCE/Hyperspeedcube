use std::collections::VecDeque;
use std::sync::Arc;

use hypermath::Isometry;
use hyperpuzzle::{LayerMask, PerPiece, Puzzle, PuzzleState, Twist};
use instant::Instant;

/// If at least this much of a twist is animated in one frame, just skip the
/// animation to reduce unnecessary flashing.
const MIN_TWIST_DELTA: f32 = 1.0 / 3.0;

/// Higher number means faster exponential increase in twist speed.
const EXP_TWIST_FACTOR: f32 = 0.5;

/// Interpolation functions.
mod interpolate {
    use std::f32::consts::PI;

    /// Function that maps a float from the range 0.0 to 1.0 to another float
    /// from 0.0 to 1.0.
    pub type InterpolateFn = fn(f32) -> f32;

    /// Interpolate using cosine from 0.0 to PI.
    pub const COSINE: InterpolateFn = |x| (1.0 - (x * PI).cos()) / 2.0;
}

use crate::preferences::Preferences;
const TWIST_INTERPOLATION_FN: interpolate::InterpolateFn = interpolate::COSINE;

#[derive(Debug, Clone)]
pub struct PuzzleController {
    /// Latest puzzle state, not including any transient rotation.
    puzzle: PuzzleState,
    /// Twist animation state.
    twist_anim: TwistAnimationState,
    /// Time of last frame, or `None` if we are not in the middle of an animation.
    last_frame_time: Option<Instant>,
}
impl PuzzleController {
    pub fn new(puzzle: &Arc<Puzzle>) -> Self {
        Self {
            puzzle: PuzzleState::new(Arc::clone(puzzle)),
            twist_anim: TwistAnimationState::default(),
            last_frame_time: None,
        }
    }

    pub fn puzzle_type(&self) -> &Arc<Puzzle> {
        self.puzzle.ty()
    }

    pub fn peice_transforms(&self) -> PerPiece<Isometry> {
        if let Some(anim) = self.twist_anim.queue.front() {
            let t = TWIST_INTERPOLATION_FN(self.twist_anim.progress);
            anim.state
                .animated_piece_transforms(anim.twist, anim.layers, t as _)
        } else {
            self.puzzle.piece_transforms()
        }
    }

    pub fn do_twist(&mut self, twist: Twist, layers: LayerMask) {
        match self.puzzle.do_twist(twist, layers) {
            Ok(new_state) => {
                let old_state = std::mem::replace(&mut self.puzzle, new_state);
                if self.puzzle.do_twist(twist, LayerMask(1)).is_ok() {
                    self.twist_anim.queue.push_back(TwistAnimation {
                        state: old_state,
                        twist,
                        layers,
                    });
                }
            }
            Err(blocking_pieces) => {
                log::error!("error executing twist!")
            }
        }
    }

    /// Advances the puzzle geometry and internal state to the next frame, using
    /// the given time delta between this frame and the last. Returns whether
    /// the puzzle must be redrawn.
    pub fn update_geometry(&mut self, prefs: &Preferences) -> bool {
        let interaction_prefs = &prefs.interaction;

        let now = Instant::now();
        let delta = match self.last_frame_time {
            Some(then) => now - then,
            None => prefs.gfx.frame_duration(),
        };

        let mut needs_redraw = false;

        // `twist_duration` is in seconds (per one twist); `base_speed` is
        // fraction of twist per frame.
        let base_speed = delta.as_secs_f32() / interaction_prefs.twist_duration;

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

        // Animate twist.
        let anim = &mut self.twist_anim;
        if anim.queue.is_empty() {
            anim.queue_max = 0;
        } else {
            // Update queue_max.
            anim.queue_max = std::cmp::max(anim.queue_max, anim.queue.len());
            // Twist exponentially faster if there are/were more twists in the
            // queue.
            let speed_mod = match interaction_prefs.dynamic_twist_speed {
                true => ((anim.queue_max - 1) as f32 * EXP_TWIST_FACTOR).exp(),
                false => 1.0,
            };
            let mut twist_delta = base_speed * speed_mod;
            // Cap the twist delta at 1.0, and also handle the case where
            // something went wrong with the calculation (e.g., division by
            // zero).
            if !(0.0..MIN_TWIST_DELTA).contains(&twist_delta) {
                twist_delta = 1.0; // Instantly complete the twist.
            }
            self.twist_anim.proceed(twist_delta);
            needs_redraw = true;
        }

        if needs_redraw {
            self.last_frame_time = Some(now);
        } else {
            self.last_frame_time = None;
        }

        needs_redraw
    }
}

#[derive(Debug, Default, Clone)]
struct TwistAnimationState {
    /// Queue of twist animations to be displayed.
    queue: VecDeque<TwistAnimation>,
    /// Maximum number of animations in the queue (reset when queue is empty).
    queue_max: usize,
    /// Progress of the animation in the current twist, from 0.0 to 1.0.
    progress: f32,
}
impl TwistAnimationState {
    fn proceed(&mut self, delta_t: f32) {
        self.progress += delta_t;
        if self.progress >= 1.0 {
            self.progress = 0.0;
            self.queue.pop_front();
        }
    }
}

#[derive(Debug, Clone)]
struct TwistAnimation {
    /// Puzzle state before twist.
    state: PuzzleState,
    /// Twist to animate.
    twist: Twist,
    /// Layers to twist.
    layers: LayerMask,
}

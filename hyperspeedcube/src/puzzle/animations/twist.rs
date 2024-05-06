use std::collections::VecDeque;

use hypermath::pga;
use hyperpuzzle::{PieceMask, PuzzleState};
use instant::Duration;

use super::interpolate;
use crate::preferences::InteractionPreferences;

/// If at least this much of a twist is animated in one frame, just skip the
/// animation to reduce unnecessary flashing.
const MIN_TWIST_DELTA: f32 = 1.0 / 3.0;

/// Higher number means faster exponential increase in twist speed.
const EXP_TWIST_FACTOR: f32 = 0.5;

const TWIST_INTERPOLATION_FN: interpolate::InterpolateFn = interpolate::COSINE;

#[derive(Debug, Default, Clone)]
pub struct TwistAnimationState {
    /// Queue of twist animations to be displayed.
    queue: VecDeque<TwistAnimation>,
    /// Maximum number of animations in the queue (reset when queue is empty).
    queue_max: usize,
    /// Progress of the animation in the current twist, from 0.0 to 1.0.
    progress: f32,
}
impl TwistAnimationState {
    /// Steps the animation forward. Returns whether the puzzle should be
    /// redrawn next frame.
    pub fn proceed(&mut self, delta: Duration, prefs: &InteractionPreferences) -> bool {
        if self.queue.is_empty() {
            self.queue_max = 0;
            false // Do not request redraw
        } else {
            // `twist_duration` is in seconds (per one twist); `base_speed` is
            // fraction of twist per frame.
            let base_speed = delta.as_secs_f32() / prefs.twist_duration;

            // Twist exponentially faster if there are/were more twists in the
            // queue.
            let speed_mod = match prefs.dynamic_twist_speed {
                true => ((self.queue_max - 1) as f32 * EXP_TWIST_FACTOR).exp(),
                false => 1.0,
            };
            let mut twist_delta = base_speed * speed_mod;
            // Cap the twist delta at 1.0, and also handle the case where
            // something went wrong with the calculation (e.g., division by
            // zero).
            if !(0.0..MIN_TWIST_DELTA).contains(&twist_delta) {
                twist_delta = 1.0; // Instantly complete the twist.
            }

            self.progress += twist_delta;
            if self.progress >= 1.0 {
                self.progress = 0.0;
                self.queue.pop_front();
            }

            true // Request redraw
        }
    }

    pub fn push(&mut self, anim: TwistAnimation) {
        self.queue.push_back(anim);

        // Update queue_max.
        self.queue_max = std::cmp::max(self.queue_max, self.queue.len());
    }

    pub fn current(&self) -> Option<(&TwistAnimation, f32)> {
        Some((self.queue.front()?, TWIST_INTERPOLATION_FN(self.progress)))
    }
}

#[derive(Debug, Clone)]
pub struct TwistAnimation {
    /// Puzzle state before the twist.
    pub state: PuzzleState,
    /// Set of pieces affected by the twist.
    pub grip: PieceMask,
    /// Initial transform of the gripped pieces (identity, unless the move was
    /// inputted using a mouse drag).
    pub initial_transform: pga::Motor,
    /// Final transform for the the gripped pieces.
    pub final_transform: pga::Motor,
}

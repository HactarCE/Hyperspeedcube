use std::collections::VecDeque;

use hyperprefs::AnimationPreferences;
use hyperpuzzle::prelude::*;
use web_time::Duration;

/// If at least this much of a twist is animated in one frame, just skip the
/// animation to reduce unnecessary flashing.
const MIN_TWIST_DELTA: f32 = 1.0 / 3.0;

/// Higher number means faster exponential increase in twist speed.
const EXP_TWIST_FACTOR: f32 = 0.5;

#[derive(Debug, Default, Clone)]
pub struct TwistAnimationState {
    /// Queue of twist animations to be displayed.
    queue: VecDeque<AnimationFromState>,
    /// Maximum number of animations in the queue (reset when queue is empty).
    queue_max: usize,
    /// Progress of the animation in the current twist, from 0.0 to 1.0.
    progress: f32,
}
impl TwistAnimationState {
    /// Steps the animation forward. Returns whether the puzzle should be
    /// redrawn next frame.
    pub fn proceed(&mut self, delta: Duration, animation_prefs: &AnimationPreferences) -> bool {
        if self.queue.is_empty() {
            self.queue_max = 0;
            false // Do not request redraw
        } else {
            // `twist_duration` is in seconds (per one twist); `base_speed` is
            // fraction of twist per frame.
            let base_speed = delta.as_secs_f32() / animation_prefs.twist_duration;

            // Twist exponentially faster if there are/were more twists in the
            // queue.
            let speed_mod = match animation_prefs.dynamic_twist_speed {
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

    pub fn push(&mut self, anim: AnimationFromState) {
        self.queue.push_back(anim);

        // Update queue_max.
        self.queue_max = std::cmp::max(self.queue_max, self.queue.len());
    }

    pub fn current(&self) -> Option<(&AnimationFromState, f32)> {
        Some((self.queue.front()?, self.progress))
    }
}

#[derive(Debug, Clone)]
pub struct AnimationFromState {
    /// Puzzle state before the animation.
    pub state: BoxDynPuzzleState,
    /// Animation to apply to the state.
    pub anim: BoxDynPuzzleAnimation,
}

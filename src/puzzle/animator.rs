//! Animation logic.

use std::collections::VecDeque;
use std::f32::consts::PI;

use super::traits::*;

const TWIST_DURATION: f32 = 0.2;
const MIN_DURATION: f32 = 0.05;

// Use cosine from 0.0 to PI for interpolation.
const INTERPOLATION_FN: fn(f32) -> f32 = |x| (1.0 - (x * PI).cos()) / 2.0;
// // Use cosine from 0.0 to PI/2.0 for interpolation.
// const INTERPOLATION_FN: fn(f32) -> f32 = |x| 1.0 - (x * PI / 2.0).cos();
// // Use cosine from PI/2.0 to 0.0 for interpolation.
// const INTERPOLATION_FN: fn(f32) -> f32 = |x| ((1.0 - x) * PI / 2.0).cos();

#[derive(Debug, Clone)]
pub struct Animator<P: PuzzleTrait> {
    /// The state of the puzzle right before the twist being animated right now.
    displayed: P,
    /// The state of the puzzle with all twists applied to it (used for timing
    /// and undo).
    latest: P,
    /// A queue of twists that transform the displayed state into the latest
    /// state.
    twists: VecDeque<P::Twist>,
    /// Maximum number of moves in the queue (reset when queue is empty).
    queue_max: usize,
    /// The progress of the animation in the current twist, from 0.0 to 1.0.
    progress: f32,
}
impl<P: PuzzleTrait> Default for Animator<P> {
    fn default() -> Self {
        Self {
            displayed: P::default(),
            latest: P::default(),
            twists: VecDeque::new(),
            queue_max: 0,
            progress: 0.0,
        }
    }
}
impl<P: PuzzleTrait> Animator<P> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn next_frame(&mut self) {
        if self.twists.is_empty() {
            self.queue_max = 0;
            return;
        }
        if self.progress == 1.0 {
            self.displayed.twist(self.twists.pop_front().unwrap());
            self.progress = 0.0;
            return;
        }
        // Update queue_max.
        self.queue_max = std::cmp::max(self.queue_max, self.twists.len());
        // TWIST_DURATION is in seconds; speed is per frame. (60 FPS)
        let base_speed = 1.0 / (TWIST_DURATION * 60.0);
        // Move exponentially faster if there are/were more moves in the queue.
        let mut speed_mod = ((self.queue_max - 1) as f32).exp();
        // Set a speed limit.
        if speed_mod > TWIST_DURATION / MIN_DURATION {
            speed_mod = TWIST_DURATION / MIN_DURATION;
        }
        let speed = base_speed * speed_mod;
        self.progress += speed;
        if self.progress >= 1.0 {
            self.progress = 1.0;
        }
    }
    pub fn displayed(&self) -> &P {
        &self.displayed
    }
    pub fn latest(&self) -> &P {
        &self.latest
    }
    pub fn twist(&mut self, twist: P::Twist) {
        self.twists.push_back(twist);
        self.latest.twist(twist);
    }
    pub fn catch_up(&mut self) {
        for twist in self.twists.drain(..) {
            self.displayed.twist(twist);
        }
        self.progress = 0.0;
        assert_eq!(self.displayed, self.latest);
    }
    pub fn current_twist(&self) -> Option<(&P::Twist, f32)> {
        if let Some(twist) = self.twists.get(0) {
            Some((twist, INTERPOLATION_FN(self.progress)))
        } else {
            None
        }
    }
}

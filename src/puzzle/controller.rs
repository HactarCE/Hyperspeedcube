//! Animation logic.

use std::collections::VecDeque;
use std::time;

use super::rubiks4d_logfile::*;
use super::{traits::*, Rubiks4D};

const TWIST_DURATION: f32 = 0.2;
const MIN_DURATION: f32 = 0.05;
const MAX_BACKLOG: usize = 10;
const INTERPOLATION_FN: InterpolateFn = interpolate::COSINE;

use cgmath::{Matrix4, SquareMatrix};
use interpolate::InterpolateFn;
/// Interpolation functions.
pub mod interpolate {
    use std::f32::consts::PI;

    /// A function that maps a float from the range 0.0 to 1.0 to another float
    /// from 0.0 to 1.0.
    pub type InterpolateFn = fn(f32) -> f32;

    /// Interpolate using cosine from 0.0 to PI.
    pub const COSINE: InterpolateFn = |x| (1.0 - (x * PI).cos()) / 2.0;
    /// Interpolate using cosine from 0.0 to PI/2.0.
    pub const COSINE_ACCEL: InterpolateFn = |x| 1.0 - (x * PI / 2.0).cos();
    /// Interpolate using cosine from PI/2.0 to 0.0.
    pub const COSINE_DECEL: InterpolateFn = |x| ((1.0 - x) * PI / 2.0).cos();
}

/// A structure to manage twists applied to a puzzle and their animation.
pub struct PuzzleController<P: PuzzleTrait> {
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

    /// Whether the puzzle has been scrambled.
    pub scramble_state: ScrambleState,
    /// Scrmable twists.
    pub scramble: Vec<P::Twist>,
    /// Undo history.
    pub undo_buffer: Vec<P::Twist>,
    /// Redo history.
    pub redo_buffer: Vec<P::Twist>,

    /// Sticker highlight filters.
    pub highlight_filter: Box<dyn Fn(P::Sticker) -> bool>,
    /// Labels.
    pub labels: Vec<(Facet<P>, String)>,
}
impl<P: PuzzleTrait> Default for PuzzleController<P> {
    fn default() -> Self {
        Self {
            displayed: P::default(),
            latest: P::default(),
            twists: VecDeque::new(),
            queue_max: 0,
            progress: 0.0,

            scramble_state: ScrambleState::default(),
            scramble: vec![],
            undo_buffer: vec![],
            redo_buffer: vec![],

            highlight_filter: Box::new(|_| true),
            labels: vec![],
        }
    }
}
impl<P: PuzzleTrait> Eq for PuzzleController<P> {}
impl<P: PuzzleTrait> PartialEq for PuzzleController<P> {
    fn eq(&self, other: &Self) -> bool {
        self.latest == other.latest
    }
}
impl<P: PuzzleTrait> PartialEq<P> for PuzzleController<P> {
    fn eq(&self, other: &P) -> bool {
        self.latest == *other
    }
}
impl<P: PuzzleTrait> PuzzleController<P> {
    /// Make a new PuzzleController with a solved puzzle.
    pub fn new() -> Self {
        Self::default()
    }
    /// Advance to the next frame, using the given time delta between this frame
    /// and the last.
    pub fn advance(&mut self, delta: time::Duration) {
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
        // TWIST_DURATION is in seconds (per one twist); speed is (fraction of twist) per frame.
        let base_speed = delta.as_secs_f32() / TWIST_DURATION;
        // Move exponentially faster if there are/were more moves in the queue.
        let mut speed_mod = ((self.queue_max - 1) as f32).exp();
        // Set a speed limit.
        if speed_mod > TWIST_DURATION / MIN_DURATION {
            speed_mod = TWIST_DURATION / MIN_DURATION;
        }
        let mut speed = base_speed * speed_mod;
        // But ignore the speed limit if we've hit max backlog.
        if self.queue_max >= MAX_BACKLOG {
            speed = 1.0;
        }
        self.progress += speed;
        if self.progress >= 1.0 {
            self.progress = 1.0;
        }
    }
    /// Returns the state of the cube that should be displayed, not including
    /// the twist currently being animated.
    pub fn displayed(&self) -> &P {
        &self.displayed
    }
    /// Returns the state of the cube after all queued twists have been applied.
    pub fn latest(&self) -> &P {
        &self.latest
    }
    /// Adds a twist to the back of the twist queue.
    pub fn twist(&mut self, twist: P::Twist) {
        self.redo_buffer.clear();
        if self.undo_buffer.last() == Some(&twist.rev()) {
            self.undo();
        } else {
            self.undo_buffer.push(twist);
            self.twists.push_back(twist);
            self.latest.twist(twist);
        }
    }
    /// Skips the animations for all twists in the queue.
    pub fn catch_up(&mut self) {
        for twist in self.twists.drain(..) {
            self.displayed.twist(twist);
        }
        self.progress = 0.0;
        assert_eq!(self.displayed, self.latest);
    }
    /// Returns the twist currently being animated, along with a float between
    /// 0.0 and 1.0 indicating the progress on that animation.
    pub fn current_twist(&self) -> Option<(P::Twist, f32)> {
        if let Some(&twist) = self.twists.get(0) {
            Some((twist, INTERPOLATION_FN(self.progress)))
        } else {
            None
        }
    }

    /// Undoes one twist.
    pub fn undo(&mut self) {
        if let Some(twist) = self.undo_buffer.pop() {
            self.redo_buffer.push(twist);
            self.twists.push_back(twist.rev());
            self.latest.twist(twist.rev());
        }
    }
    /// Redoes one twist.
    pub fn redo(&mut self) {
        if let Some(twist) = self.redo_buffer.pop() {
            self.undo_buffer.push(twist);
            self.twists.push_back(twist);
            self.latest.twist(twist);
        }
    }
}

impl PuzzleController<Rubiks4D> {
    /// Loads a log file and returns the puzzle state.
    pub fn load_file(path: &str) -> Result<Self, String> {
        let contents = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let logfile = contents
            .parse::<super::rubiks4d_logfile::LogFile>()
            .map_err(|e| e.to_string())?;

        let mut ret = Self::default();
        ret.scramble_state = logfile.scramble_state;
        for twist in logfile.scramble_twists {
            ret.twist(twist);
        }
        ret.scramble = ret.undo_buffer;
        ret.undo_buffer = vec![];
        ret.catch_up();
        for twist in logfile.solve_twists {
            ret.twist(twist);
        }

        Ok(ret)
    }
    /// Saves the puzzle state to a log file.
    pub fn save_file(&self, path: &str) -> Result<(), String> {
        let logfile = LogFile {
            scramble_state: self.scramble_state,
            view_matrix: Matrix4::identity(),
            scramble_twists: self.scramble.clone(),
            solve_twists: self.undo_buffer.clone(),
        };

        std::fs::write(path, logfile.to_string()).map_err(|e| e.to_string())
    }
}

/// Whether the puzzle has been scrambled.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ScrambleState {
    /// Unscrambled.
    None = 0,
    /// Some small number of scramble twists.
    Partial = 1,
    /// Fully scrambled.
    Full = 2,
    /// Was solved by user even if not currently solved.
    Solved = 3,
}
impl Default for ScrambleState {
    fn default() -> Self {
        ScrambleState::None
    }
}

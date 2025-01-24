use std::sync::atomic::{AtomicBool, AtomicU32};

use rand::Rng;

use super::{LayeredTwist, PuzzleState};
use crate::Timestamp;

/// Parameters to deterministically generate a twist sequence to scramble a
/// puzzle.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ScrambleParams {
    /// Type of scramble to generate.
    pub ty: ScrambleType,
    /// Timestamp when the scramble was requested.
    pub time: Timestamp,
    /// Random seed, probably sourced from a "true" RNG provided by the OS.
    pub seed: u32,
}
impl ScrambleParams {
    /// Generates a new random scramble based on the current time.
    pub fn new(ty: ScrambleType) -> Self {
        Self {
            ty,
            time: Timestamp::now(),
            seed: rand::rng().random(),
        }
    }
}

/// Type of scramble to generate.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ScrambleType {
    /// Full scramble.
    Full,
    /// Partial scramble of a specific number of moves.
    Partial(u32),
}

/// Progress while scrambling.
#[derive(Debug)]
pub struct ScrambleProgress {
    done: AtomicU32,
    total: AtomicU32,
    cancel_requested: AtomicBool,
    // output: Mutex<Option<(Vec<LayeredTwist>, PuzzleState)>>,
}
impl Default for ScrambleProgress {
    fn default() -> Self {
        Self {
            done: AtomicU32::new(0),
            total: AtomicU32::new(1),
            cancel_requested: AtomicBool::new(false),
            // output: Mutex::new(None),
        }
    }
}
impl ScrambleProgress {
    /// Constructs a new `ScrambleProgress`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the progress as a fraction: completed moves / total moves.
    pub fn fraction(&self) -> (u32, u32) {
        (
            self.done.load(std::sync::atomic::Ordering::Relaxed),
            self.total.load(std::sync::atomic::Ordering::Relaxed),
        )
    }
    pub(super) fn set_total(&self, total: u32) {
        self.total
            .store(total, std::sync::atomic::Ordering::Relaxed);
    }
    pub(super) fn set_progress(&self, twists_done: u32) {
        self.done
            .store(twists_done, std::sync::atomic::Ordering::Relaxed);
    }

    /// Requests to cancel the scrambling.
    pub fn request_cancel(&self) {
        self.cancel_requested
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
    pub(super) fn is_cancel_requested(&self) -> bool {
        self.cancel_requested
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    // pub(super) fn set_output(&self, output: (Vec<LayeredTwist>, PuzzleState)) {
    //     *self.output.lock() = Some(output);
    // }
    // pub fn try_take_output(&self) -> Option<(Vec<LayeredTwist>, PuzzleState)> {
    //     self.output.lock().take()
    // }
}

/// Output of scrambling a puzzle.
pub struct ScrambledPuzzle {
    /// Parameters used to generate the scramble.
    pub params: ScrambleParams,
    /// Scramble twists applied.
    pub twists: Vec<LayeredTwist>,
    /// State of the puzzle after scrambling.
    pub state: PuzzleState,
}

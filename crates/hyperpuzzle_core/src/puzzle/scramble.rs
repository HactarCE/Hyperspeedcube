use std::sync::atomic::{AtomicBool, AtomicU32};

use rand::Rng;
use serde::{Deserialize, Serialize};

use super::{LayeredTwist, PuzzleState};
use crate::Timestamp;

/// Parameters to deterministically generate a twist sequence to scramble a
/// puzzle.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ScrambleParams {
    /// Type of scramble to generate.
    pub ty: ScrambleType,
    /// Timestamp when the scramble was requested.
    pub time: Timestamp,
    /// Random seed, probably sourced from a "true" RNG provided by the OS or by
    /// the leaderboard server.
    pub seed: String,
}
impl ScrambleParams {
    /// Generates a new random scramble based on the current time.
    pub fn new(ty: ScrambleType) -> Self {
        let time = Timestamp::now();
        let random_u64: u64 = rand::rng().random();
        let seed = format!("{time}_{random_u64}");
        Self { ty, time, seed }
    }
}

/// Type of scramble to generate.
#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
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

#[cfg(feature = "timecheck")]
use std::ops::Range;
use std::sync::atomic::{AtomicBool, AtomicU32};

#[cfg(feature = "timecheck")]
use chrono::{DateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use timecheck::drand::DrandRound;

use super::LayeredTwist;
use crate::{BoxDynPuzzleState, Timestamp};

/// Parameters to deterministically generate a twist sequence to scramble a
/// puzzle.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ScrambleParams {
    /// Type of scramble to generate.
    pub ty: ScrambleType,
    /// Timestamp when the scramble was requested.
    pub time: Timestamp,
    /// Random seed, probably sourced from a "true" RNG provided by the OS or by
    /// a randomness beacon.
    pub seed: String,

    /// Randomness beacon round used to generate the seed.
    #[cfg(feature = "timecheck")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drand_round_v1: Option<DrandRound>,
}
impl ScrambleParams {
    /// Generates a new scramble based on the current time and a random number.
    pub fn new(ty: ScrambleType) -> Self {
        let time = Timestamp::now();
        let seed = Self::seed_from_time_and_u64(time, rand::rng().random());
        Self {
            ty,
            time,
            seed,

            #[cfg(feature = "timecheck")]
            drand_round_v1: None,
        }
    }

    /// Generates a new scramble based on the current time and the output of a
    /// public randomness beacon.
    ///
    /// **This function blocks while waiting for a response from the randomness
    /// beacon.**
    #[cfg(feature = "timecheck")]
    pub fn from_randomness_beacon(ty: ScrambleType) -> timecheck::drand::Result<Self> {
        let drand = timecheck::drand::Drand {
            chain: crate::get_drand_chain(),
            ..Default::default()
        };
        let drand_round = drand.get_latest_randomness_round()?;

        let time = Timestamp::now();
        let seed = Self::seed_from_time_and_bytes(time, &drand_round.signature);
        Ok(Self {
            ty,
            time,
            seed,

            drand_round_v1: Some(drand_round),
        })
    }

    /// Verifies that the scramble was generated from the specified randomness
    /// beacon and returns the time range during which the scramble was likely
    /// generated. It is effectively impossible for the scramble to have been
    /// generated before the range, and if it was generated after the range then
    /// it should have used a more up-to-date random value. Returns `None` if
    /// the scramble was not generated from a randomness beacon.
    ///
    /// **This method blocks and should be run on a background thread.**
    #[cfg(feature = "timecheck")]
    pub fn verify_from_randomness_beacon(
        &self,
    ) -> Result<Range<DateTime<Utc>>, ScrambleVerificationError> {
        // Check randomness source
        let drand_round = self
            .drand_round_v1
            .as_ref()
            .ok_or(ScrambleVerificationError::Offline)?;
        let drand_chain = crate::get_drand_chain();
        drand_chain.verify(drand_round)?;

        // Check seed
        let expected_seed = Self::seed_from_time_and_bytes(self.time, &drand_round.signature);
        if expected_seed != self.seed {
            return Err(ScrambleVerificationError::SeedDoesNotMatch);
        }

        Ok(drand_chain.round_time_range(drand_round.number)?)
    }

    fn seed_from_time_and_u64(time: Timestamp, random_u64: u64) -> String {
        format!("{time}_{random_u64}")
    }
    #[cfg(feature = "timecheck")]
    fn seed_from_time_and_bytes(time: Timestamp, bytes: &[u8]) -> String {
        use base64::prelude::*;
        use sha2::Digest;

        let sha256 = sha2::Sha256::digest(bytes);
        let base64_encoded_sha256 = BASE64_STANDARD.encode(sha256);
        format!("{time}_{base64_encoded_sha256}")
    }
}

#[cfg(feature = "timecheck")]
#[derive(thiserror::Error, Serialize, Deserialize, Debug)]
#[allow(missing_docs)]
pub enum ScrambleVerificationError {
    #[error("offline scramble")]
    Offline,
    #[error("drand error: {0}")]
    Drand(String),
    #[error("scramble seed does not match")]
    SeedDoesNotMatch,
}
#[cfg(feature = "timecheck")]
impl From<timecheck::drand::DrandError> for ScrambleVerificationError {
    fn from(value: timecheck::drand::DrandError) -> Self {
        Self::Drand(value.to_string())
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
    pub state: BoxDynPuzzleState,
}

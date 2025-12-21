use std::sync::atomic::{AtomicBool, AtomicU32};

use rand::Rng;
use serde::{Deserialize, Serialize};

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

    /// Randomness beacon used to generate the seed.
    pub beacon_url: Option<String>,
    /// Randomness beacon round number used to generate the seed.
    pub beacon_round: Option<u64>,
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

            beacon_url: None,
            beacon_round: None,
        }
    }

    /// Generates a new scramble based on the current time and the output of a
    /// public randomness beacon.
    ///
    /// This function blocks while waiting for a response from the randomness
    /// beacon.
    ///
    /// See <https://docs.drand.love/developer/> for a list of possible beacon
    /// URLs. It is recommended to use a URL from
    /// [`TRUSTED_RANDOMNESS_BEACON_URLS`], because those are the only ones
    /// accepted by [`ScrambleParams::verify_from_randomness_beacon()`].
    #[cfg(feature = "drand")]
    pub fn from_randomness_beacon(
        ty: ScrambleType,
        beacon_url: &str,
    ) -> Result<Self, drand_core::DrandError> {
        let client = drand_core::HttpClient::new(beacon_url, None)?;
        let beacon_output = client.latest()?;
        let random_bytes = beacon_output.randomness();

        let time = Timestamp::now();
        let seed = Self::seed_from_time_and_bytes(time, &random_bytes);
        Ok(Self {
            ty,
            time,
            seed,

            beacon_url: Some(client.base_url()),
            beacon_round: Some(beacon_output.round()),
        })
    }

    /// Verifies that the scramble was generated from the specified randomness
    /// beacon. Returns `Ok(None)` if the scramble was not generated from a
    /// randomness beacon, `Ok(Some(true))` if the scramble matches,
    /// `Ok(Some(false))` if the scramble does not match, or `Err(..)` if there
    /// was a network error.
    ///
    /// This function blocks while waiting for a response from the randomness
    /// beacon.
    ///
    /// **Do not call this method unless the randomness beacon URL is trusted.**
    #[cfg(feature = "drand")]
    pub fn verify_from_randomness_beacon(&self) -> Result<Option<bool>, drand_core::DrandError> {
        let Some(beacon_url) = &self.beacon_url else {
            return Ok(None);
        };
        let client = drand_core::HttpClient::new(beacon_url, None)?;
        let beacon_output = client.latest()?;
        let random_bytes = beacon_output.randomness();

        let expected_seed = Self::seed_from_time_and_bytes(self.time, &random_bytes);
        Ok(Some(expected_seed == self.seed))
    }

    fn seed_from_time_and_u64(time: Timestamp, random_u64: u64) -> String {
        format!("{time}_{random_u64}")
    }
    #[cfg(feature = "drand")]
    fn seed_from_time_and_bytes(time: Timestamp, bytes: &[u8]) -> String {
        use base64::prelude::*;

        let base64_encoded_bytes = BASE64_STANDARD.encode(bytes);
        format!("{time}_{base64_encoded_bytes}")
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

/// Output of verifying a timestamp (randomness beacon or time stamping
/// authority).
#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
enum NetworkVerifiedTimestamp {
    /// Evidence is proven to be good.
    Verified {
        /// Earliest possible timestamp.
        earliest: Timestamp,
        /// Latest possible timestamp.
        latest: Timestamp,
    },
    /// Evidence is proven to be bad.
    VerifiedBad,
    /// Network error; evidence is indeterminate.
    NetworkError,
    /// Evidence relies on an untrusted URL, so it was not queried.
    UntrustedUrl,
    /// No evidence.
    NoData,
}

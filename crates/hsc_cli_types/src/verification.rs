//! Types used by the `verify` subcommand.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// Verified info about a solve of a puzzle.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SolveVerification {
    /// Canonical ID for the puzzle, which determines its leaderboard category.
    pub puzzle_canonical_id: String,
    /// Puzzle version number, as a string.
    pub puzzle_version: String,
    /// Number of moves in the solution.
    pub solution_stm: u64,
    /// Whether the solution used piece filters.
    pub used_filters: bool,
    /// Whether the solution used macros.
    pub used_macros: bool,

    /// Timestamps of various events, according to the log file.
    pub timestamps: Timestamps,
    /// Timestamps of various events that were able to be cryptographically
    /// verified with a third party.
    pub verified_timestamps: VerifiedTimestamps,
    /// Durations of various time intervals.
    ///
    /// This is `None` if the solve was not a valid speedsolve or was completed
    /// over multiple sessions.
    pub durations: Durations,

    /// Errors reported during verification.
    pub errors: Vec<String>,
}

/// Timestamps of various events, according to the log file.
#[derive(Serialize, Deserialize, Debug, Default, Copy, Clone, PartialEq, Eq)]
pub struct Timestamps {
    /// Time that the scramble was generated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scramble_generation: Option<DateTime<Utc>>,
    /// Time that the puzzle was done being scrambled and was presented to the
    /// user. For large puzzles, this may take a significant amount of time
    /// compared to when the scramble was generated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inspection_start: Option<DateTime<Utc>>,
    /// Final time that blindfold mode was enabled, if this is a valid
    /// blindsolve.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blindfold_don: Option<DateTime<Utc>>,
    /// Time that the first move was applied to the puzzle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub solve_start: Option<DateTime<Utc>>,
    /// Time that the solve ended.
    ///
    /// - For blindfolded solves, this is the time that the blindfold was lifted
    ///   after the puzzle was solved.
    /// - For ordinary speedsolves, this is the time that the last move was
    ///   applied to the puzzle, solving it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub solve_completion: Option<DateTime<Utc>>,
}

/// Timestamps of various events that were able to be cryptographically verified
/// with a third party.
#[derive(Serialize, Deserialize, Debug, Default, Copy, Clone, PartialEq, Eq)]
pub struct VerifiedTimestamps {
    /// Earliest time the scramble **could** have been generated.
    ///
    /// It is effectively impossible for the scramble to have been generated
    /// before this time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scramble_range_start: Option<DateTime<Utc>>,
    /// Latest time the scramble **should** have been generated.
    ///
    /// Scrambles generated after this time should have used a more up-to-date
    /// random value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scramble_range_end: Option<DateTime<Utc>>,
    /// Latest time the solve could have been completed.
    ///
    /// It is effectively impossible for the solve to have been completed or
    /// tampered with after this time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completion: Option<DateTime<Utc>>,
}

/// Durations of various events.
#[derive(Serialize, Deserialize, Debug, Default, Copy, Clone, PartialEq, Eq)]
pub struct Durations {
    /// Duration between `verified_timestamps.scramble_range_end` and
    /// `timestamps.scramble_generation`.
    ///
    /// This is the network latency from the randomness beacon to the client.
    /// Due to precision of the randomness beacon, this may be negative.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scramble_network_latency: Option<Duration>,
    /// Duration between `timestamps.scramble_generation` and
    /// `timestamps.inspection_start`.
    ///
    /// This is the time taken to apply the scramble to the puzzle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scramble_application: Option<Duration>,

    /// Duration between `timestamps.inspection_start` and
    /// `timestamps.first_move`, or `None` if the solve is a blindsolve.
    ///
    /// This is the time taken for inspection. `None` if the solve is a
    /// blindsolve.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inspection: Option<Duration>,
    /// Duration between `timestamps.first_move` and `timestamps.last_move`.
    /// `None` if the solve is a blindsolve.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speedsolve: Option<Duration>,

    /// Duration between `timestamps.inspection_start` and
    /// `timestamps.blindfold_don`
    ///
    /// `None` if the solve is not a blindsolve.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<Duration>,
    /// Duration between `timestamps.blindfold_doff` and
    /// `timestamps.inspection_start`.
    ///
    /// `None` if the solve is not a blindsolve.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blindsolve: Option<Duration>,

    /// Duration between `timestamps.solve_completion` and
    /// `verified_timestamps.completion`.
    ///
    /// This is the network latency from the client to the time stamp authority.
    /// Due to precision of the time stamp authority, this may be negative.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp_network_latency: Option<Duration>,
}

impl Durations {
    /// Computes durations from timestamps.
    pub fn new(
        timestamps: Timestamps,
        verified_timestamps: VerifiedTimestamps,
        is_valid_blindsolve: bool,
    ) -> Self {
        // IIFE to mimic try_block
        Self {
            scramble_network_latency: (|| {
                Some(timestamps.scramble_generation? - verified_timestamps.scramble_range_end?)
            })(),
            scramble_application: (|| {
                Some(timestamps.inspection_start? - timestamps.scramble_generation?)
            })(),

            inspection: (|| Some(timestamps.solve_start? - timestamps.inspection_start?))(),
            speedsolve: (|| Some(timestamps.solve_completion? - timestamps.solve_start?))(),

            memo: (|| Some(timestamps.blindfold_don? - timestamps.inspection_start?))()
                .filter(|_| is_valid_blindsolve),
            blindsolve: (|| Some(timestamps.solve_completion? - timestamps.inspection_start?))()
                .filter(|_| is_valid_blindsolve),

            timestamp_network_latency: (|| {
                Some(verified_timestamps.completion? - timestamps.solve_completion?)
            })(),
        }
    }
}

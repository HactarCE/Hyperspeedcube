//! Functions for verifying log files.

use hyperpuzzle_core::chrono::Utc;
use hyperpuzzle_core::prelude::*;
use hyperpuzzle_core::{Timestamp, chrono};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use super::*;

pub const TRUSTED_RANDOMNESS_BEACON_URLS: &[&str] = &["api.drand.sh", "api2.drand.sh", "api3.drand.sh"];

/// Which properties of a solve to verify.
#[derive(Serialize, Deserialize, Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct VerificationOptions {
    /// Construct the puzzle and verify that the puzzle is solved after applying
    /// the scramble + solution.
    pub verify_solution: bool,
    /// If the scramble was derived using randomness from a randomness
    /// beacon, verify the randomness.
    pub verify_randomness_from_beacon: bool,
    /// If the solution is timestamped using a Time Stamping Authority, verify
    /// the timestamp.
    pub verify_completion_timestamp: bool,
}
impl VerificationOptions {
    /// Perform all checks, including those that may be expensive or require a network connection.
    pub const FULL: Self = Self {
        verify_solution: true,
        verify_randomness_from_beacon: true,
        verify_completion_timestamp: true,
    };

    /// Skip all checks that may be computationally expensive or require network
    /// access. Use this only for solves that are already completely trusted.
    pub const QUICK: Self = Self {
        verify_solution: false,
        verify_randomness_from_beacon: false,
        verify_completion_timestamp: false,
    };
}

/// Verifies a log file.
///
/// This function blocks while constructing/simulating a puzzle and doing
/// network requests.
pub fn verify(
    catalog: &Catalog,
    solve: &Solve,
    options: VerificationOptions,
) -> Option<SolveVerification> {
    if !solve.solved {
        return None;
    }
    let scramble = solve.scramble.clone()?;
    let scramble_params = scramble.params()?;

    if scramble_params.ty!=ScrambleType::Full {
        return None;
    }

    log::info!("building puzzle {} for verification", solve.puzzle.id);
    let puzzle = match catalog.build_blocking::<Puzzle>(&solve.puzzle.id) {
        Ok(p) => p,
        Err(e) => {
            log::error!("error building puzzle {}: {e}", solve.puzzle.id);
            return None;
        }
    };

    let scramble_twists: Vec<LayeredTwist> =
        notation::parse_twists(&puzzle.twists.names, &scramble.twists)
            .try_collect()
            .ok()?;
    let expected_scrambled_puzzle = puzzle.new_scrambled(scramble_params.clone()); // TODO: this may be very slow
    let is_scramble_correct = expected_scrambled_puzzle.twists == scramble_twists;

    let mut log = solve.log.iter();

    let Some(LogEvent::Scramble { .. }) = log.next() else {
        return None; // didn't start by scrambling!
    };

    let mut undo_stack: Vec<SmallVec<[LayeredTwist; 1]>> = vec![];
    let mut redo_stack: Vec<SmallVec<[LayeredTwist; 1]>> = vec![];
    let mut time_completed = None;
    let mut speedsolve_start = None;
    let mut speedsolve_end = None;
    let mut single_session = true;
    for event in log {
        match event {
            LogEvent::Scramble { .. } => return None, // don't scramble again!
            LogEvent::Click { .. } | LogEvent::DragTwist { .. } => (), // ignore interaction events
            LogEvent::Twists(twists_str) => {
                for twist_group in notation::parse_grouped_twists(&puzzle.twists.names, twists_str)
                {
                    undo_stack.push(twist_group.into_iter().try_collect().ok()?);
                }
            }
            LogEvent::Undo { .. } => redo_stack.push(undo_stack.pop()?),
            LogEvent::Redo { .. } => undo_stack.push(redo_stack.pop()?),
            LogEvent::StartSolve { time: _, duration } => {
                speedsolve_start = *duration;
            }
            LogEvent::EndSolve { time, duration } => {
                time_completed = *time;
                speedsolve_end = *duration;
                break; // apparently we're done!
            }
            LogEvent::StartSession { .. } | LogEvent::EndSession { .. } => {
                single_session = false;
            }
        }
    }
    let twist_groups = undo_stack;
    let time_completed = time_completed?; // must say when it was completed

    let mut twists_done;
    if options.verify_solution {
        let mut puzzle_state = puzzle.new_solved_state();
        for twist in scramble_twists {
            if let Ok(new_state) = puzzle_state.do_twist_dyn(twist) {
                puzzle_state = new_state;
            }
        }
        twists_done = vec![];
        for twist in twist_groups.into_iter().flatten() {
            if let Ok(new_state) = puzzle_state.do_twist_dyn(twist) {
                puzzle_state = new_state;
                twists_done.push(twist);
            }
        }
        if !puzzle_state.is_solved() {
            return None;
        }
    } else {
        twists_done = twist_groups.into_iter().flatten().collect();
    }
    let solution_stm_count = TwistMetric::Stm.count_twists(&puzzle, twists_done);

    let speedsolve_duration = Option::zip(speedsolve_start, speedsolve_end)
        .and_then(|(start, end)| end.checked_sub(start))
        .map(chrono::Duration::milliseconds);

    // IIFE to mimic try_block
    let scramble_timestamp_verified = (|| {

        if !options.verify_randomness_from_beacon {
             None
        }else

        if let Some(beacon_url)= scramble_params.beacon_url&& !TRUSTED_RANDOMNESS_BEACON_URLS.contains(&beacon_url) {
             Some(NetworkVerifiedTimestamp::UntrustedUrl)
        }

        else{match scramble_params.verify_from_randomness_beacon() {
            Ok(Some(true)) => Some(NetworkVerifiedTimestamp::Verified { earliest: (), latest: () }),
            Ok(Some(false)) => Some(NetworkVerifiedTimestamp::VerifiedBad),
            Ok(None) => Some(NetworkVerifiedTimestamp::NoData),
            Err(_) => Some(NetworkVerifiedTimestamp::NetworkError),
        }}

        let Ok(beacon_output) = drand_core::HttpClient::new(beacon_url, None)
            .and_then(|client| client.get(beacon_round))
        else {
            return NetworkVerifiedTimestamp::NetworkError;
        };
        if scramble_params.seed == format!("{}", scramble.time)
        beacon_output.randomness()
    })();

    let completion_timestamp_verified = options.verify_completion_timestamp && { todo!() };

    Some(SolveVerification {
        puzzle: solve.puzzle.clone(),
        scramble: scramble_params,
        is_scramble_correct,
        solution_stm_count,
        single_session,
        used_macros: false, // not yet implemented
        inspection_duration: speedsolve_start.map(chrono::Duration::milliseconds),
        speedsolve_duration,
        blindsolve_duration: None, // not yet implemented
        time_completed,
        scramble_timestamp_verified,
        completion_timestamp_verified,
    })
}

/// Fact learned from verifying a log file.
pub enum Fact {
    /// A puzzle has been scrambled and then completely solved.
    Solve(SolveVerification),
}

/// Info about a scramble and solve of a puzzle.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SolveVerification {
    /// Puzzle that was solved.
    pub puzzle: LogPuzzle,
    /// Parameters used to determine the scramble.
    ///
    /// You may want to check that the type is [`ScrambleType::Full`].
    pub scramble: ScrambleParams,

    /// Number of twists in the solution, measured using [Slice Turn
    /// Metric](https://hypercubing.xyz/notation/#turn-metrics).
    pub solution_stm_count: u64,
    /// Whether the solve was completed within a single session.
    pub single_session: bool,
    /// Whether any macros were used in the solution.
    pub used_macros: bool,
    /// Duration of the inspection part of the solve.
    ///
    /// The timer starts when the puzzle has been scrambled and ends on the
    /// first move.
    pub inspection_duration: Option<chrono::Duration>,
    /// Duration of the solve measured as a speedsolve, or `None` if it was not
    /// a valid speedsolve.
    ///
    /// The timer starts on the first move and ends when the puzzle is visible &
    /// solved.
    pub speedsolve_duration: Option<chrono::Duration>,
    /// Duration of the solve measured as a blindsolve, or `None` if it was not
    /// a valid blindsolve.
    ///
    /// The timer starts when the puzzle has been scrambled and ends when the
    /// puzzle is visible & solved.
    pub blindsolve_duration: Option<chrono::Duration>,
    /// Timestamp when the solve was completed.
    pub time_completed: Timestamp,

    /// Whether the moves in the scramble match the one specified by the
    /// parameters. `None` if not checked.
    pub is_scramble_correct: Option<bool>,
    /// Whether the scramble timestamp was able to be verified with a randomness
    /// beacon. `None` if not checked.
    pub scramble_timestamp_verified: Option<NetworkVerifiedTimestamp>,
    /// Whether the completion timestamp was able to be verified with a
    /// timestamping authority. `None` if not checked.
    pub completion_timestamp_verified: Option<NetworkVerifiedTimestamp>,
}

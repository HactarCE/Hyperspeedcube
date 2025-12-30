//! Functions for verifying log files.

use hyperpuzzle_core::prelude::*;
use hyperpuzzle_core::{Timestamp, chrono};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::notation::TwistParseError;

use super::*;

const TSA: timecheck::tsa::Tsa = timecheck::tsa::Tsa::FREETSA;

/// Which properties of a solve to verify.
#[derive(Serialize, Deserialize, Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct VerificationOptions {
    /// Construct the puzzle and verify that the scramble matches the scramble
    /// seed.
    pub verify_scramble: bool,
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
        verify_scramble: true,
        verify_solution: true,
        verify_randomness_from_beacon: true,
        verify_completion_timestamp: true,
    };

    /// Skip all checks that may be computationally expensive or require network
    /// access. Use this only for solves that are already completely trusted.
    pub const QUICK: Self = Self {
        verify_scramble: false,
        verify_solution: false,
        verify_randomness_from_beacon: false,
        verify_completion_timestamp: false,
    };
}

/// Timestamps the solve as having been completed by the current time.
///
/// **This function blocks while waiting for a response from the Time Stamp
/// Authority.**
pub fn timestamp(solve: &mut Solve) -> timecheck::tsa::Result<()> {
    let digest = solve.digest_v1();
    let signature = TSA.timestamp(&digest)?;
    solve.tsa_signature_v1 = Some(signature.to_string());
    Ok(())
}

/// Verifies a log file.
///
/// This function blocks while constructing/simulating a puzzle and doing
/// network requests.
pub fn verify(
    catalog: &Catalog,
    solve: &Solve,
    options: VerificationOptions,
) -> Result<SolveVerification, SolveVerificationError> {
    if !solve.solved {
        return Err(SolveVerificationError::NotSolved);
    }
    let scramble = solve
        .scramble
        .clone()
        .ok_or(SolveVerificationError::NoScramble)?;
    let scramble_params = scramble
        .params()
        .ok_or(SolveVerificationError::NondeterministicScramble)?;

    if scramble_params.ty != ScrambleType::Full {
        return Err(SolveVerificationError::NotFullyScrambled);
    }

    log::info!("building puzzle {} for verification", solve.puzzle.id);
    let puzzle = match catalog.build_blocking::<Puzzle>(&solve.puzzle.id) {
        Ok(p) => p,
        Err(e) => {
            log::error!("error building puzzle {}: {e}", solve.puzzle.id);
            return Err(SolveVerificationError::PuzzleBuildError(e));
        }
    };

    let scramble_twists: Vec<LayeredTwist>;
    let is_scramble_correct = if options.verify_scramble {
        scramble_twists =
            notation::parse_twists(&puzzle.twists.names, &scramble.twists).try_collect()?;
        let expected_scrambled_puzzle = puzzle.new_scrambled(scramble_params.clone()); // TODO: this may be very slow
        Some(expected_scrambled_puzzle.twists == scramble_twists)
    } else {
        scramble_twists = vec![];
        None
    };

    let mut log = solve.log.iter().peekable();

    log.next_if(|entry| matches!(entry, LogEvent::StartSession { .. }));

    let Some(LogEvent::Scramble { .. }) = log.next() else {
        return Err(SolveVerificationError::DoesntStartWithScramble);
    };

    let mut undo_stack: Vec<SmallVec<[LayeredTwist; 1]>> = vec![];
    let mut redo_stack: Vec<SmallVec<[LayeredTwist; 1]>> = vec![];
    let mut time_completed = None;
    let mut speedsolve_start = None;
    let mut speedsolve_end = None;
    let mut single_session = true;
    for event in log {
        match event {
            LogEvent::Scramble { .. } => return Err(SolveVerificationError::DoubleScramble), // don't scramble again!
            LogEvent::Click { .. } | LogEvent::DragTwist { .. } => (), // ignore interaction events
            LogEvent::Twists(twists_str) => {
                for twist_group in notation::parse_grouped_twists(&puzzle.twists.names, twists_str)
                {
                    undo_stack.push(twist_group.into_iter().try_collect()?);
                }
            }
            LogEvent::Undo { .. } => {
                redo_stack.push(undo_stack.pop().ok_or(SolveVerificationError::UndoError)?)
            }
            LogEvent::Redo { .. } => {
                undo_stack.push(redo_stack.pop().ok_or(SolveVerificationError::RedoError)?)
            }
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
    let time_completed = time_completed.ok_or(SolveVerificationError::NoCompletionTime)?; // must say when it was completed

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
            return Err(SolveVerificationError::NotSolved);
        }
    } else {
        twists_done = twist_groups.into_iter().flatten().collect();
    }
    let solution_stm_count = TwistMetric::Stm.count_twists(&puzzle, twists_done);

    let speedsolve_duration = Option::zip(speedsolve_start, speedsolve_end)
        .and_then(|(start, end)| end.checked_sub(start))
        .map(chrono::Duration::milliseconds);

    let mut errors = vec![];

    let scramble_timestamp_range = if options.verify_randomness_from_beacon {
        match scramble_params.verify_from_randomness_beacon() {
            Ok(range) => Some([range.start, range.end].map(Timestamp)),
            Err(e) => {
                errors.push(format!("cannot validate scramble timestamp: {e}"));
                None
            }
        }
    } else {
        None
    };

    let completion_timestamp = if options.verify_completion_timestamp {
        let timestamp_result = if let Some(signature) = &solve.tsa_signature_v1 {
            timecheck::tsa::Signature::from_str(&signature)
                .map_err(|e| timecheck::tsa::TsaError::Other(e.into()))
                .and_then(|sig| {
                    let expected_digest = solve.digest_v1();
                    TSA.verify(&expected_digest, &sig)?;
                    Ok(Timestamp(sig.timestamp()?))
                })
        } else {
            Err(timecheck::tsa::TsaError::Other("no signature".into()))
        };
        match timestamp_result {
            Ok(timestamp) => Some(timestamp),
            Err(e) => {
                errors.push(format!("cannot validate completion timestamp: {e}"));
                None
            }
        }
    } else {
        None
    };

    Ok(SolveVerification {
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
        verified_scramble_timestamp_range: scramble_timestamp_range,
        verified_completion_timestamp: completion_timestamp,
        errors,
    })
}

#[derive(thiserror::Error, Debug, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum SolveVerificationError {
    #[error("not solved")]
    NotSolved,
    #[error("no scramble")]
    NoScramble,
    #[error("nondeterministic scramble")]
    NondeterministicScramble,
    #[error("not fully scrambled")]
    NotFullyScrambled,
    #[error("doesn't start with scramble")]
    DoesntStartWithScramble,
    #[error("scrambled multiple times")]
    DoubleScramble,
    #[error("puzzle build error: {0}")]
    PuzzleBuildError(String),
    #[error("puzzle build error: {0}")]
    TwistParseError(String),
    #[error("undo stack underflow")]
    UndoError,
    #[error("redo stack underflow")]
    RedoError,
    #[error("no completion time")]
    NoCompletionTime,
}

impl<'a> From<TwistParseError<'a>> for SolveVerificationError {
    fn from(value: TwistParseError<'a>) -> Self {
        Self::TwistParseError(value.to_string())
    }
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
    /// Timestamp range from the Randomness Beacon. `None` if not checked.
    ///
    /// The scramble must have been generated at or after this time.
    pub verified_scramble_timestamp_range: Option<[Timestamp; 2]>,

    /// Timestamp from the Time Stamp Authority. `None` if not checked.
    ///
    /// The solve must have been complted at or before this time.
    pub verified_completion_timestamp: Option<Timestamp>,

    /// List of errors describing why some verification may have failed.
    pub errors: Vec<String>,
}

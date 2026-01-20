//! Functions for verifying log files.

use hyperpuzzle_core::prelude::*;
use hyperpuzzle_core::verification::*;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use super::*;
use crate::notation::TwistParseError;

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
    /// Perform all checks, including those that may be expensive or require a
    /// network connection.
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

/// Returns the timestamp signature for a solve that marks it as having been
/// completed by the current time.
///
/// **This function blocks while waiting for a response from the Time Stamp
/// Authority.**
pub fn timestamp(digest: &[u8]) -> timecheck::tsa::Result<String> {
    Ok(TSA.timestamp(digest)?.to_string())
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
    let mut errors: Vec<String> = vec![];

    let is_replay = solve.replay.unwrap_or(false);

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
    if options.verify_scramble {
        scramble_twists =
            notation::parse_twists(&puzzle.twists.names, &scramble.twists).try_collect()?;
        let expected_scrambled_puzzle = puzzle.new_scrambled(scramble_params.clone()); // TODO: this may be very slow
        if expected_scrambled_puzzle.twists != scramble_twists {
            return Err(SolveVerificationError::ScrambleSeedMismatch);
        }
    } else {
        scramble_twists = vec![];
    };

    let mut log = solve.log.iter().peekable();

    let Some(&LogEvent::Scramble {
        time: scramble_generation,
    }) = log.next()
    else {
        return Err(SolveVerificationError::DoesntStartWithScramble);
    };

    let inspection_start;
    if let Some(&&LogEvent::StartSession { time }) = log.peek() {
        inspection_start = time;
        log.next();
    } else {
        inspection_start = None;
    }

    let mut undo_stack: Vec<SmallVec<[LayeredTwist; 1]>> = vec![];
    let mut redo_stack: Vec<SmallVec<[LayeredTwist; 1]>> = vec![];
    let mut blindfold_don = None;
    let mut solve_start = None;
    let mut solve_completion = None;
    let mut single_session = true;
    let mut bld_fsm = BlindsolveFsm::default();
    let mut used_filters = false;
    let mut used_macros = false;
    for event in log {
        match event {
            LogEvent::Scramble { .. } => return Err(SolveVerificationError::DoubleScramble), /* don't scramble again */
            LogEvent::Click { .. } | LogEvent::DragTwist { .. } => (), // ignore interaction events
            LogEvent::Twists(twists_str) => {
                bld_fsm.do_twist();
                for twist_group in notation::parse_grouped_twists(&puzzle.twists.names, twists_str)
                {
                    undo_stack.push(twist_group.into_iter().try_collect()?);
                    redo_stack.clear();
                }
            }
            LogEvent::Undo { .. } => {
                if let Some(twist) = undo_stack.pop() {
                    redo_stack.push(twist);
                }
            }
            LogEvent::Redo { .. } => {
                if let Some(twist) = redo_stack.pop() {
                    undo_stack.push(twist);
                }
            }
            LogEvent::SetBlindfold { time, enabled } => {
                bld_fsm.set_blindfold_state(*enabled);
                if *enabled {
                    blindfold_don = *time;
                }
            }
            LogEvent::InvalidateFilterless { .. } => used_filters = true,
            LogEvent::Macro { .. } => {
                bld_fsm.do_twist();
                used_macros = true;
                let e = "macros are not supported in this version of `hyperpuzzle_log`".to_string();
                if !errors.contains(&e) {
                    errors.push(e);
                }
            }
            LogEvent::StartSolve { time, .. } => solve_start = *time,
            LogEvent::EndSolve { time, .. } => {
                solve_completion = *time;
                break; // apparently we're done!
            }
            LogEvent::StartSession { .. } | LogEvent::EndSession { .. } => single_session = false,
        }
    }
    let twist_groups = undo_stack;

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
    let solution_stm = TwistMetric::Stm.count_twists(&puzzle, twists_done);

    let is_valid_blindsolve = bld_fsm == BlindsolveFsm::SolveComplete;

    let timestamps = Timestamps {
        scramble_generation: scramble_generation.map(|t| t.0),
        inspection_start: inspection_start.map(|t| t.0),
        blindfold_don: blindfold_don.map(|t| t.0).filter(|_| is_valid_blindsolve),
        solve_start: solve_start.map(|t| t.0),
        solve_completion: solve_completion.map(|t| t.0),
    };

    let verified_timestamps = {
        let [scramble_range_start, scramble_range_end] = if options.verify_randomness_from_beacon {
            match scramble_params.verify_from_randomness_beacon() {
                Ok(range) => [Some(range.start), Some(range.end)],
                Err(e) => {
                    errors.push(format!("cannot validate scramble timestamp: {e}"));
                    [None; 2]
                }
            }
        } else {
            [None; 2]
        };

        let completion = if options.verify_completion_timestamp {
            let timestamp_result = if let Some(signature) = &solve.tsa_signature_v1 {
                timecheck::tsa::Signature::from_str(signature)
                    .map_err(|e| timecheck::tsa::TsaError::Other(e.into()))
                    .and_then(|sig| {
                        let expected_digest = solve.digest_v1();
                        TSA.verify(&expected_digest, &sig)?;
                        sig.timestamp()
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

        VerifiedTimestamps {
            scramble_range_start,
            scramble_range_end,
            completion,
        }
    };

    Ok(SolveVerification {
        puzzle_canonical_id: solve.puzzle.id.clone(),
        puzzle_version: solve.puzzle.version.clone(),
        solution_stm,
        used_filters,
        used_macros,

        timestamps,
        verified_timestamps,
        durations: if is_replay && single_session {
            Durations::new(timestamps, verified_timestamps, is_valid_blindsolve)
        } else {
            Durations::default() // empty
        },

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
    #[error("scramble does not match seed")]
    ScrambleSeedMismatch,
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

/// Finite state machine for validating blindsolves.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
enum BlindsolveFsm {
    /// Blindfold off, no moves done
    #[default]
    InspectionBlindfoldOff,
    /// Blindfold on, no moves done
    InspectionBlindfoldOn,
    /// Blindfold on, some moves done
    Solving,
    /// Blindfold off, solve is complete
    SolveComplete,
    /// Not a valid blindsolve
    Invalid,
}

impl BlindsolveFsm {
    fn do_twist(&mut self) {
        match self {
            Self::InspectionBlindfoldOff => *self = Self::Invalid,
            Self::InspectionBlindfoldOn => *self = Self::Solving,
            Self::Solving => (),
            Self::SolveComplete => *self = Self::Invalid,
            Self::Invalid => (),
        }
    }

    fn set_blindfold_state(&mut self, new_state: bool) {
        match new_state {
            true => self.don_blindfold(),
            false => self.doff_blindfold(),
        }
    }

    fn don_blindfold(&mut self) {
        match self {
            BlindsolveFsm::InspectionBlindfoldOff => *self = Self::InspectionBlindfoldOn,
            BlindsolveFsm::InspectionBlindfoldOn => *self = Self::Invalid,
            BlindsolveFsm::Solving => *self = Self::Invalid,
            BlindsolveFsm::SolveComplete => *self = Self::Invalid,
            BlindsolveFsm::Invalid => (),
        }
    }

    fn doff_blindfold(&mut self) {
        match self {
            BlindsolveFsm::InspectionBlindfoldOff => *self = Self::Invalid,
            BlindsolveFsm::InspectionBlindfoldOn => *self = Self::InspectionBlindfoldOff,
            BlindsolveFsm::Solving => *self = Self::SolveComplete,
            BlindsolveFsm::SolveComplete => *self = Self::Invalid,
            BlindsolveFsm::Invalid => (),
        }
    }
}

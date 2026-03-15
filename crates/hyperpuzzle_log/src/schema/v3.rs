use hyperkdl::{DocSchema, Warning};
use hyperpuzzle_core::{LayerMask, ScrambleParams, ScrambleType, Timestamp};
use itertools::Itertools;
use kdl::*;
use serde::{Deserialize, Serialize};
use sha2::Digest;

use crate::{Error, KdlProxy};

/// Log file version. This **MUST** be incremented whenever breaking changes are
/// made to the log file format.
pub const LOG_FILE_VERSION: i128 = 3;

/// Deserializes a log file from a string.
pub fn deserialize(mut doc: KdlDocument, warnings: &mut Vec<Warning>) -> Result<LogFile, Error> {
    if let Some((version, _)) = crate::remove_kdl_version_number(&mut doc)
        && version != LOG_FILE_VERSION
    {
        return Err(Error::UnsupportedVersion);
    }

    Ok(LogFile::from_kdl_doc(&doc, hyperkdl::DeserCtx::new(warnings)).unwrap_or_default())
}

/// Top-level log file structure.
///
/// A single log file may contain multiple solves.
#[derive(Debug, Default, Clone, PartialEq, Eq, hyperkdl_derive::Doc)]
pub struct LogFile {
    /// Information about the software that created the log file.
    #[kdl(child("program"), optional)]
    pub program: Option<Program>,
    /// List of solves.
    #[kdl(children)]
    pub solves: Vec<Solve>,
}
impl LogFile {
    /// Serializes the log file to a string.
    pub fn serialize(&self) -> String {
        let mut doc = self.to_kdl_doc();
        doc.set_format(KdlDocumentFormat {
            leading: "// Hyperspeedcube puzzle log\n".to_owned(),
            trailing: String::new(),
        });

        // version
        doc.nodes_mut().insert(0, {
            let mut node = KdlNode::new("version");
            node.push(KdlEntry::new(LOG_FILE_VERSION));
            node
        });

        doc.autoformat();

        doc.to_string()
    }
}

/// Information about the software that created the log file.
#[derive(Debug, Clone, PartialEq, Eq, hyperkdl_derive::NodeContents)]
pub struct Program {
    /// Name of the program.
    #[kdl(property("name"), optional)]
    pub name: Option<String>,
    /// Version number.
    ///
    /// - There should be no leading `v`.
    /// - The version number should ideally follow [Semantic Versioning](https://semver.org/)
    ///   with respect to the contents of the log file.
    #[kdl(property("version"), optional)]
    pub version: Option<String>,
}

/// Solve of a puzzle.
#[derive(Debug, Clone, PartialEq, Eq, hyperkdl_derive::Node, hyperkdl_derive::NodeContents)]
#[kdl(name = "solve")]
pub struct Solve {
    /// Whether the log includes replay events. Default `false`.
    #[kdl(child("replay"), optional)]
    pub replay: Option<bool>,

    /// Puzzle info.
    ///
    /// This is the only part of a solve that is strictly required.
    #[kdl(child("puzzle"))]
    pub puzzle: LogPuzzle,
    /// Whether the puzzle has been solved from a scramble.
    ///
    /// This is always `false` if the puzzle has not been fully scrambled. If at
    /// any time the puzzle is in the solved state and the state is visible
    /// (i.e., not blindfolded), then this flag is set to `true`. It remains
    /// `true` even if additional moves are performed that make it unsolved. It
    /// reverts to `false` if moves are _undone_ to make it no longer solved.
    ///
    /// This corresponds exactly to whether there is a [`LogEvent::EndSolve`] in
    /// `log` that has _not_ been undone.
    #[kdl(child("solved"), default = false)]
    pub solved: bool,
    /// Number of milliseconds that the log has been open for, across all
    /// sessions.
    #[kdl(child("duration"), optional)]
    pub duration: Option<i64>,

    // pub macros: (),
    // pub keybinds: (),
    // pub vantages: (),
    // pub filters: (),
    /// Scramble.
    ///
    /// This is applied to the puzzle with [`LogEvent::Scramble`].
    #[kdl(child("scramble"), optional)]
    pub scramble: Option<Scramble>,
    /// List of events.
    ///
    /// If the log includes replay events, then this is a linear history of all
    /// events in time, even if they were later undone.
    ///
    /// If the log does not include replay events, then undone events are not
    /// included.
    #[kdl(child("log"))]
    pub log: Vec<LogEvent>,
    /// Time Stamp Authority signature using [`Solve::digest_v1()`].
    #[kdl(child("tsa_signature_v1"), optional)]
    pub tsa_signature_v1: Option<String>,
    /// Time Stamp Authority signature using [`Solve::digest_v2()`].
    #[kdl(child("tsa_signature_v2"), optional)]
    pub tsa_signature_v2: Option<String>,
    /// Time Stamp Authority signature using [`Solve::digest_v3()`].
    #[kdl(child("tsa_signature_v3"), optional)]
    pub tsa_signature_v3: Option<String>,
}

impl Solve {
    /// Returns a SHA-256 digest of the events of the solve, in JSON.
    #[deprecated]
    pub fn digest_v1(&self) -> Vec<u8> {
        let serialized_log =
            serde_json::to_string(&self.log).expect("error serializing log for time stamping");
        sha2::Sha256::digest(&serialized_log).as_slice().to_vec()
    }
    /// Due to a bug, this always hashes only the scramble time. Do not use this
    /// method.
    #[deprecated]
    pub fn digest_v2(&self) -> Vec<u8> {
        let log_events = self.log.first().into_iter().collect_vec();
        let serialized_log =
            serde_json::to_string(&log_events).expect("error serializing log for time stamping");
        sha2::Sha256::digest(&serialized_log).as_slice().to_vec()
    }
    /// Returns a SHA-256 digest of the events of the solve, in JSON.
    ///
    /// Changes from v1:
    ///
    /// - If there is an `EndSolve` event, then all events after it are
    ///   excluded. For typical speedsolves, this excludes the "end session"
    ///   event at the end (whose timestamp may vary) and excludes any moves
    ///   done after the solve is completed.
    ///
    /// This allows the digest to remain the same even if the log file is loaded
    /// and saved again or moves are done after the end of the solve.
    pub fn digest_v3(&self) -> Vec<u8> {
        let log_events = self.digest_v3_events().collect_vec();
        let serialized_log =
            serde_json::to_string(&log_events).expect("error serializing log for time stamping");
        sha2::Sha256::digest(&serialized_log).as_slice().to_vec()
    }

    pub(crate) fn digest_v3_events(&self) -> impl Iterator<Item = &LogEvent> {
        self.log
            .iter()
            .take_while_inclusive(|ev| !matches!(ev, LogEvent::EndSolve { .. }))
    }
}

/// Puzzle info.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, hyperkdl_derive::NodeContents)]
pub struct LogPuzzle {
    /// Puzzle ID.
    #[kdl(property("id"))]
    pub id: String,
    /// Puzzle version number.
    ///
    /// - There must be no leading `v`.
    /// - The version number should ideally follow [Semantic Versioning](https://semver.org/)
    ///   with respect to the contents of the log file.
    #[kdl(property("version"))]
    pub version: String,
}

/// Scramble info.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, hyperkdl_derive::NodeContents)]
pub struct Scramble {
    /// Scramble type selected by the user.
    #[kdl(argument, proxy = KdlProxy)]
    pub ty: ScrambleType,
    /// Scramble algorithm version.
    #[kdl(property("version"))]
    pub version: u32,
    /// Optional timestamp at which the scramble was generated.
    #[kdl(property("time"), optional, proxy = KdlProxy)]
    pub time: Option<Timestamp>,
    /// Optional random seed which, in conjunction with the timestamp,
    /// deterministically generates the twists sequence.
    ///
    /// If this is not present, then the scramble is assumed to have been
    /// generated nondeterministically.
    #[kdl(property("seed"), optional)]
    pub seed: Option<String>,
    /// Twist sequence to apply to the puzzle, using standard notation.
    #[kdl(child("twists"))]
    pub twists: String,
    /// Randomness beacon round.
    #[kdl(child("drand_round_v1"), optional)]
    pub drand_round_v1: Option<DrandRound>,
}
impl Scramble {
    /// Returns the parameters used to deterministically generated the twist
    /// sequence.
    ///
    /// Returns `None` if the scramble was generated nondeterministically.
    pub fn params(&self) -> Option<ScrambleParams> {
        Some(ScrambleParams {
            version: self.version,
            ty: self.ty,
            time: self.time?,
            seed: self.seed.clone()?,
            drand_round_v1: self
                .drand_round_v1
                .as_ref()
                .and_then(|round| round.to_timecheck_drand_round()),
        })
    }
    /// Constructs a scramble from scramble parameters and a twist sequence.
    pub fn new(params: ScrambleParams, twists: String) -> Self {
        Self {
            version: params.version,
            ty: params.ty,
            time: Some(params.time),
            seed: Some(params.seed),
            twists,
            drand_round_v1: params
                .drand_round_v1
                .as_ref()
                .map(DrandRound::from_timecheck_drand_round),
        }
    }
}

/// Randomness data fetched from a randomness beacon to seed the scramble.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, hyperkdl_derive::NodeContents)]
pub struct DrandRound {
    /// Round number.
    #[kdl(property("round"))]
    pub round: i64,
    /// Signature from which the scramble is derived.
    #[kdl(property("signature"))]
    pub signature: String,
    /// Previous signature (empty if unchained).
    #[kdl(property("previous_signature"), optional)]
    pub previous_signature: Option<String>,
}
impl DrandRound {
    /// Converts to [`timecheck::drand::DrandRound`].
    pub fn to_timecheck_drand_round(&self) -> Option<timecheck::drand::DrandRound> {
        Some(timecheck::drand::DrandRound {
            number: self.round as u64,
            signature: hex::decode(&self.signature).ok()?,
            previous_signature: hex::decode(self.previous_signature.as_deref().unwrap_or_default())
                .ok()?,
        })
    }
    /// Converts from [`timecheck::drand::DrandRound`].
    pub fn from_timecheck_drand_round(round: &timecheck::drand::DrandRound) -> Self {
        Self {
            round: round.number as i64,
            signature: hex::encode(&round.signature),
            previous_signature: Some(hex::encode(&round.previous_signature))
                .filter(|s| !s.is_empty()),
        }
    }
}

/// Event in a solve log.
#[derive(Serialize, Debug, Clone, PartialEq, Eq, hyperkdl_derive::Node)]
#[serde(rename_all = "snake_case")]
pub enum LogEvent {
    /// Application of the scramble sequence.
    #[kdl(name = "scramble")]
    Scramble {
        /// Event timestamp.
        #[kdl(property("time"), optional, proxy = KdlProxy)]
        #[serde(skip_serializing_if = "Option::is_none")]
        time: Option<Timestamp>,
    },
    /// **Replay-only.** Click of the mouse cursor on the puzzle.
    #[kdl(name = "click")]
    Click {
        /// Event timestamp.
        #[kdl(property("time"), optional, proxy = KdlProxy)]
        #[serde(skip_serializing_if = "Option::is_none")]
        time: Option<Timestamp>,
        /// Layer mask gripped.
        #[kdl(property("layers"), proxy = KdlProxy)]
        layers: LayerMask,
        /// String identifier for the area clicked.
        #[kdl(property("target"))]
        target: String,
        /// Whether a reverse click was performed.
        ///
        /// By convention, right mouse button typically performs a forward click
        /// and left mouse button typically performs a reverse click.
        #[kdl(property("reverse"), default)]
        #[serde(skip_serializing_if = "std::ops::Not::not")]
        reverse: bool,
    },
    /// **Replay-only.** Drag of the mouse cursor on the puzzle to execute a
    /// twist. If no twist was executed, this event does not need to be
    /// recorded.
    #[kdl(name = "drag-twist")]
    DragTwist {
        /// Event timestamp.
        #[kdl(property("time"), optional, proxy = KdlProxy)]
        #[serde(skip_serializing_if = "Option::is_none")]
        time: Option<Timestamp>,
        /// Axis that was twisted.
        #[kdl(property("axis"))]
        axis: String,
    },
    /// Sequence of twists separated by spaces.
    ///
    /// Twists grouped using parentheses were executed as a single action, and
    /// are undone/redone as a single action.
    #[kdl(name = "twists")]
    Twists(#[kdl(argument)] String),
    /// **Replay-only.** Undo of the most recent twist, twist group, or macro.
    #[kdl(name = "undo")]
    Undo {
        /// Event timestamp.
        #[kdl(property("time"), optional, proxy = KdlProxy)]
        #[serde(skip_serializing_if = "Option::is_none")]
        time: Option<Timestamp>,
    },
    /// **Replay-only.** Redo of the most recent twist, twist group, or macro.
    #[kdl(name = "redo")]
    Redo {
        /// Event timestamp.
        #[kdl(property("time"), optional, proxy = KdlProxy)]
        #[serde(skip_serializing_if = "Option::is_none")]
        time: Option<Timestamp>,
    },
    /// **Replay-only.** Set blindfolded state.
    #[kdl(name = "set-blindfold")]
    SetBlindfold {
        /// Event timestamp.
        #[kdl(property("time"), optional, proxy = KdlProxy)]
        #[serde(skip_serializing_if = "Option::is_none")]
        time: Option<Timestamp>,
        /// New blindfolded state.
        #[kdl(property("on"))]
        enabled: bool,
    },
    /// **Replay-only.** Invalidate a no-filters speedsolve.
    #[kdl(name = "invalidate-filterless")]
    InvalidateFilterless {
        /// Event timestamp.
        #[kdl(property("time"), optional, proxy = KdlProxy)]
        #[serde(skip_serializing_if = "Option::is_none")]
        time: Option<Timestamp>,
    },
    /// Macro invocation.
    #[kdl(name = "macro")]
    Macro {
        /// Event timestamp.
        #[kdl(property("time"), optional, proxy = KdlProxy)]
        #[serde(skip_serializing_if = "Option::is_none")]
        time: Option<Timestamp>,
        // TODO: more info
    },
    /// Start of solve.
    ///
    /// This marks the first time that a twist was made on the puzzle after
    /// scrambling.
    ///
    /// This cannot be undone, and so may only appear at most once in a log.
    #[kdl(name = "start-solve")]
    StartSolve {
        /// Timestamp at which the solve started.
        #[kdl(property("time"), optional, proxy = KdlProxy)]
        #[serde(skip_serializing_if = "Option::is_none")]
        time: Option<Timestamp>,
        /// Number of milliseconds that the log had been open for, across all
        /// sessions, at the moment the solve started.
        #[kdl(property("duration"))]
        #[serde(skip_serializing_if = "Option::is_none")]
        duration: Option<i64>,
    },
    /// End of solve.
    ///
    /// This marks the first time that the puzzle reached a solved state and the
    /// state was visible (i.e., not blindfolded).
    ///
    /// This may appear multiple times in a replay file if the final twist was
    /// undone and then the puzzle was later solved.
    #[kdl(name = "end-solve")]
    EndSolve {
        /// Timestamp at which the solve ended.
        #[kdl(property("time"), optional, proxy = KdlProxy)]
        #[serde(skip_serializing_if = "Option::is_none")]
        time: Option<Timestamp>,
        /// Number of milliseconds that the log had been open for, across all
        /// sessions, at the moment the solve ended.
        #[kdl(property("duration"))]
        #[serde(skip_serializing_if = "Option::is_none")]
        duration: Option<i64>,
    },
    /// **Replay-only.** Beginning of session.
    ///
    /// This marks the start of the log and times when the log was loaded.
    #[kdl(name = "start-session")]
    StartSession {
        /// Timestamp at which the session started.
        #[kdl(property("time"), optional, proxy = KdlProxy)]
        #[serde(skip_serializing_if = "Option::is_none")]
        time: Option<Timestamp>,
    },
    /// **Replay-only.** End of session.
    ///
    /// This marks when the log file was saved.
    #[kdl(name = "end-session")]
    EndSession {
        /// Timestamp at which the session ended.
        #[kdl(property("time"), optional, proxy = KdlProxy)]
        #[serde(skip_serializing_if = "Option::is_none")]
        time: Option<Timestamp>,
    },
}

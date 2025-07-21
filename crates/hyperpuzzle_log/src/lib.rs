//! Structures and serialization/deserialization for puzzle twist notation and
//! log files.

#[macro_use]
extern crate lazy_static;

use std::str::FromStr;

use hyperkdl::{DocSchema, ValueSchemaProxy, Warning};
use hyperpuzzle_core::{LayerMask, ScrambleParams, ScrambleType, Timestamp};
use kdl::*;
use serde::{Deserialize, Serialize};

pub mod notation;
pub mod verify;

/// Log file version. This **MUST** be incremented whenever breaking changes are
/// made to the log file format.
pub const LOG_FILE_VERSION: i128 = 2;

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

    /// Deserializes a log file from a string.
    pub fn deserialize(s: &str) -> Result<(Self, Vec<Warning>), KdlError> {
        let mut doc = KdlDocument::from_str(s)?;

        // Reject if no version number
        let Some(version_node) = doc
            .nodes_mut()
            .iter_mut()
            .position(|node| node.name().value() == "version")
            .map(|i| doc.nodes_mut().remove(i))
        else {
            return Ok((
                Self::default(),
                vec![Warning {
                    span: doc.span(),
                    msg: "missing log file format version number".to_owned(),
                }],
            ));
        };
        let Some(version_number) =
            (|| version_node.entries().iter().next()?.value().as_integer())()
        else {
            return Ok((
                Self::default(),
                vec![Warning {
                    span: version_node.span(),
                    msg: "invalid log file format version number".to_owned(),
                }],
            ));
        };

        let mut warnings = vec![];

        // Check version number
        if version_number > LOG_FILE_VERSION {
            warnings.push(Warning {
                span: version_node.span(),
                msg: "this file was saved using a newer version, and might not load correctly"
                    .to_owned(),
            });
        }

        Ok((
            Self::from_kdl_doc(&doc, hyperkdl::DeserCtx::new(&mut warnings)).unwrap_or_default(),
            warnings,
        ))
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
    /// If the log includes replay events, then this includes a linear history
    /// of all events in time, even if they were later undone.
    ///
    /// If the log does not include replay events, then undone events are not
    /// included.
    #[kdl(child("log"))]
    pub log: Vec<LogEvent>,
}
/// Puzzle info.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, hyperkdl_derive::NodeContents)]
pub struct LogPuzzle {
    /// Puzzle ID.
    #[kdl(property("id"))]
    pub id: String,
    /// Puzzle version number.
    ///
    /// - There should be no leading `v`.
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
}
impl Scramble {
    /// Returns the parameters used to deterministically generated the twist
    /// sequence.
    ///
    /// Returns `None` if the scramble was generated nondeterministically.
    pub fn params(&self) -> Option<ScrambleParams> {
        Some(ScrambleParams {
            ty: self.ty,
            time: self.time?,
            seed: self.seed.clone()?,
        })
    }
    /// Constructs a scramble from scramble parameters and a twist sequence.
    pub fn new(params: ScrambleParams, twists: String) -> Self {
        Self {
            ty: params.ty,
            time: Some(params.time),
            seed: Some(params.seed),
            twists,
        }
    }
}

/// Event in a solve log.
#[derive(Debug, Clone, PartialEq, Eq, hyperkdl_derive::Node)]
pub enum LogEvent {
    /// Application of the scramble sequence.
    #[kdl(name = "scramble")]
    Scramble,
    /// **Replay-only.** Click of the mouse cursor on the puzzle.
    #[kdl(name = "click")]
    Click {
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
        reverse: bool,
    },
    /// **Replay-only.** Drag of the mouse cursor on the puzzle to execute a
    /// twist. If no twist was executed, this event does not need to be
    /// recorded.
    #[kdl(name = "drag-twist")]
    DragTwist {
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
    Undo,
    /// **Replay-only.** Redo of the most recent twist, twist group, or macro.
    #[kdl(name = "redo")]
    Redo,
    /// Start of solve.
    ///
    /// This marks the first time that a twist was made on the puzzle after
    /// scrambling.
    ///
    /// This cannot be undone, and so may only appear at most once in a log.
    #[kdl(name = "start-solve")]
    StartSolve {
        /// Timestamp at which the solve started.
        #[kdl(property("time"), proxy = KdlProxy)]
        time: Option<Timestamp>,
        /// Number of milliseconds that the log had been open for, across all
        /// sessions, at the moment the solve started.
        #[kdl(property("duration"))]
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
        #[kdl(property("time"), proxy = KdlProxy)]
        time: Option<Timestamp>,
        /// Number of milliseconds that the log had been open for, across all
        /// sessions, at the moment the solve ended.
        #[kdl(property("duration"))]
        duration: Option<i64>,
    },
    /// **Replay-only.** Beginning of session.
    ///
    /// This marks the start of the log and times when the log was loaded.
    #[kdl(name = "start-session")]
    StartSession {
        /// Timestamp at which the session started.
        #[kdl(property("time"), proxy = KdlProxy)]
        time: Option<Timestamp>,
    },
    /// **Replay-only.** End of session.
    ///
    /// This marks when the log file was saved.
    #[kdl(name = "end-session")]
    EndSession {
        /// Timestamp at which the session ended.
        #[kdl(property("time"), proxy = KdlProxy)]
        time: Option<Timestamp>,
    },
}

/// KDL serialization proxy type for some types defined in `hyperpuzzle_core`.
pub struct KdlProxy;
impl ValueSchemaProxy<LayerMask> for KdlProxy {
    fn proxy_from_kdl_value(value: &KdlValue) -> Option<LayerMask> {
        Some(LayerMask(u32::try_from(value.as_integer()?).ok()?))
    }
    fn proxy_to_kdl_value(value: &LayerMask) -> KdlValue {
        KdlValue::Integer(i128::from(value.0))
    }
}
impl ValueSchemaProxy<Timestamp> for KdlProxy {
    fn proxy_from_kdl_value(value: &KdlValue) -> Option<Timestamp> {
        Timestamp::from_str(value.as_string()?).ok()
    }
    fn proxy_to_kdl_value(value: &Timestamp) -> KdlValue {
        KdlValue::String(value.to_string())
    }
}
impl ValueSchemaProxy<ScrambleType> for KdlProxy {
    fn proxy_from_kdl_value(value: &KdlValue) -> Option<ScrambleType> {
        match value {
            KdlValue::String(s) => match s.as_str() {
                "full" => Some(ScrambleType::Full),
                _ => None,
            },
            KdlValue::Integer(n) => u32::try_from(*n).ok().map(ScrambleType::Partial),
            _ => None,
        }
    }
    fn proxy_to_kdl_value(value: &ScrambleType) -> KdlValue {
        match *value {
            ScrambleType::Full => KdlValue::from("full"),
            ScrambleType::Partial(n) => KdlValue::from(i128::from(n)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_puzzle_log_export() {
        let log_file = LogFile {
            program: Some(Program {
                name: Some("Hyperspeedcube".to_string()),
                version: Some("2.0.0-pre.15".to_string()),
            }),
            solves: vec![Solve {
                puzzle: LogPuzzle {
                    id: "ft_cube:3".to_string(),
                    version: "1.0.0".to_string(),
                },
                solved: true,
                duration: Some(5 * 60 * 1000),
                scramble: Some(Scramble {
                    ty: ScrambleType::Partial(3),
                    time: Some(Timestamp::now()),
                    seed: Some("abc".to_string()),
                    twists: "R U L'".to_string(),
                }),
                log: vec![
                    LogEvent::Scramble,
                    LogEvent::Twists("L U' R'".to_string()),
                    LogEvent::EndSolve {
                        time: Some(Timestamp::now()),
                        duration: Some(3000),
                    },
                    LogEvent::EndSession {
                        time: Some(Timestamp::now()),
                    },
                ],
            }],
        };
        std::thread::sleep(std::time::Duration::from_millis(10)); // force timestamp to change
        let serialized = log_file.serialize();
        println!("{serialized}");
        std::thread::sleep(std::time::Duration::from_millis(10)); // force timestamp to change
        let (deserialized, _warnings) = LogFile::deserialize(&serialized).unwrap();
        assert_eq!(log_file, deserialized);
    }
}

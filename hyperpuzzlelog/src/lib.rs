//! Structures and serialization/deserialization for puzzle twist notation and
//! log files.

#![allow(missing_docs)]

#[macro_use]
extern crate lazy_static;

use std::str::FromStr;

use hyperpuzzle::{LayerMask, ScrambleInfo, ScrambleType, Timestamp};
use kdl::*;

pub mod notation;

/// Log file version. This **MUST** be incremented whenever breaking changes are
/// made to the log file format.
pub const LOG_FILE_VERSION: i64 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogFile {
    pub program: Option<Program>,
    pub solves: Vec<Solve>,
}
impl LogFile {
    pub fn serialize(&self) -> String {
        let Self { program, solves } = self;

        let mut doc = KdlDocument::new();
        let root_nodes = doc.nodes_mut();

        // version
        root_nodes.push({
            let mut node = KdlNode::new("version");
            node.set_leading("// Hyperspeedcube puzzle log\n");
            node.push(KdlEntry::new(LOG_FILE_VERSION));
            node
        });

        // program
        if let Some(program) = program {
            root_nodes.push(program.to_kdl());
        }

        // solves
        for solve in solves {
            root_nodes.push(solve.to_kdl());
        }

        doc.to_string()
    }

    pub fn deserialize(s: &str) -> Result<Self, KdlError> {
        let mut program = None;
        let mut solves = vec![];

        let doc = KdlDocument::from_str(s)?;
        for node in doc.nodes() {
            match node.name().value() {
                "program" => program = Some(Program::from_kdl(node)),
                "solve" => {
                    // ignore invalid
                    if let Some(solve) = Solve::from_kdl(node) {
                        solves.push(solve);
                    }
                }
                _ => (), // ignore unknown
            }
        }

        Ok(Self { program, solves })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub name: Option<String>,
    pub version: Option<String>,
}
impl Program {
    fn to_kdl(&self) -> KdlNode {
        let Self { name, version } = self;
        let mut node = KdlNode::new("program");
        if let Some(name) = name {
            node.push(("name", name.as_str()));
        }
        if let Some(version) = version {
            node.push(("version", version.as_str()));
        }
        node
    }
    fn from_kdl(node: &KdlNode) -> Self {
        let mut name = None;
        let mut version = None;
        for entry in node.entries() {
            match entry.name().map(|name| name.value()) {
                Some("name") => name = Some(entry.value().to_string()),
                Some("version") => version = Some(entry.value().to_string()),
                _ => (), // ignore unknown
            }
        }
        Self { name, version }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Solve {
    pub puzzle: Puzzle,
    pub solved: bool,
    pub duration: Option<i64>, // milliseconds
    // pub macros: (),
    // pub keybinds: (),
    // pub vantages: (),
    // pub filters: (),
    pub scramble: Option<Scramble>,
    pub log: Vec<LogEvent>,
}
impl Solve {
    fn to_kdl(&self) -> KdlNode {
        let mut children = KdlDocument::new();
        let nodes = children.nodes_mut();

        let Self {
            puzzle,
            solved,
            duration,
            scramble,
            log,
        } = self;

        // puzzle
        nodes.push(puzzle.to_kdl());

        // duration
        if let Some(duration) = duration {
            let mut node = KdlNode::new("duration");
            node.push(*duration);
            nodes.push(node);
        }

        // scramble
        if let Some(scramble) = scramble {
            nodes.push(scramble.to_kdl());
        }

        // solved
        if scramble.is_some() {
            nodes.push({
                let mut node = KdlNode::new("solved");
                node.push(*solved);
                node
            });
        }

        nodes.push({
            let mut node = KdlNode::new("log");
            set_children_to_events_list(&mut node, &log);
            node
        });

        let mut node = KdlNode::new("solve");
        node.set_children(children);
        node
    }
    fn from_kdl(node: &KdlNode) -> Option<Solve> {
        let mut puzzle = None;
        let mut solved = false;
        let mut duration = None;
        let mut scramble = None;
        let mut log = vec![];

        for child in node.children()?.nodes() {
            match child.name().value() {
                "puzzle" => puzzle = Puzzle::from_kdl(child),
                // IIFE to mimic try_block
                "solved" => {
                    solved = (|| child.entries().first()?.value().as_bool())().unwrap_or(false);
                }
                "duration" => {
                    // IIFE to mimic try_block
                    duration = (|| child.entries().first()?.value().as_i64())();
                }
                "scramble" => scramble = Scramble::from_kdl(child),
                "log" => {
                    if let Some(children) = child.children() {
                        log.extend(children.nodes().iter().filter_map(LogEvent::from_kdl));
                    }
                }
                _ => (), // ignore unknown
            }
        }

        Some(Solve {
            puzzle: puzzle?,
            solved,
            duration,
            scramble,
            log,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Puzzle {
    pub id: String,
    pub version: String,
}
impl Puzzle {
    fn to_kdl(&self) -> KdlNode {
        let Self { id, version } = self;
        let mut node = KdlNode::new("puzzle");
        node.push(("id", id.as_str()));
        node.push(("version", version.as_str()));
        node
    }
    fn from_kdl(node: &KdlNode) -> Option<Self> {
        let mut id = None;
        let mut version = None;
        for entry in node.entries() {
            match entry.name().map(|name| name.value()) {
                Some("id") => id = entry.value().as_string(),
                Some("version") => version = entry.value().as_string(),
                _ => (), // ignore unknown
            }
        }
        Some(Self {
            id: id.map(|s| s.to_owned())?,
            version: version.map(|s| s.to_owned())?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scramble {
    pub info: ScrambleInfo,
    pub twists: String,
}
impl Scramble {
    fn to_kdl(&self) -> KdlNode {
        let Self {
            info: ScrambleInfo { ty, time, seed },
            twists,
        } = self;
        let mut node = KdlNode::new("scramble");
        node.push(match *ty {
            ScrambleType::Full => KdlValue::from("full"),
            ScrambleType::Partial(n) => KdlValue::from(n as i64),
        });
        node.push(("time", time.to_string()));
        node.push(("seed", *seed as i64));
        set_children_to_events_list(&mut node, &[LogEvent::Twists(twists.to_owned())]);
        node
    }
    fn from_kdl(node: &KdlNode) -> Option<Self> {
        let mut ty = None;
        let mut time = None;
        let mut seed = None;
        let mut twists = String::new();
        for entry in node.entries() {
            let value = entry.value();
            match entry.name().map(|name| name.value()) {
                None => {
                    ty = match value {
                        KdlValue::RawString(s) | KdlValue::String(s) if s == "full" => {
                            Some(ScrambleType::Full)
                        }
                        KdlValue::Base2(n)
                        | KdlValue::Base8(n)
                        | KdlValue::Base10(n)
                        | KdlValue::Base16(n) => u32::try_from(*n).ok().map(ScrambleType::Partial),
                        _ => None,
                    };
                }
                Some("time") => time = value.as_string().and_then(|s| Timestamp::from_str(s).ok()),
                Some("seed") => seed = value.as_i64().and_then(|i| u32::try_from(i).ok()),
                _ => (), // ignore unknown
            }
        }
        for child in node.children()?.nodes() {
            match LogEvent::from_kdl(child) {
                Some(LogEvent::Twists(new_twists)) => {
                    if !twists.is_empty() {
                        twists += " ";
                    }
                    twists.push_str(&new_twists);
                }
                _ => (), // ignore invalid
            }
        }
        Some(Self {
            info: ScrambleInfo {
                ty: ty?,
                time: time?,
                seed: seed?,
            },
            twists,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogEvent {
    Scramble,
    Click {
        layers: LayerMask,
        target: String,
        reverse: bool,
    },
    Twists(String),
    EndSolve {
        time: Timestamp,
    },
    EndSession {
        time: Timestamp,
    },
}
impl LogEvent {
    fn to_kdl(&self) -> KdlNode {
        let mut node;
        match self {
            LogEvent::Scramble => node = KdlNode::new("scramble"),

            LogEvent::Click {
                layers,
                target,
                reverse,
            } => {
                node = KdlNode::new("click");
                if !layers.is_default() {
                    node.push(("layers", i64::from(layers.0)));
                }
                node.push(("target", target.as_str()));
                if *reverse {
                    node.push(("reverse", true));
                }
            }

            LogEvent::Twists(twist_string) => {
                node = KdlNode::new("twists");
                node.push(twist_string.clone());
            }

            LogEvent::EndSolve { time } => {
                node = KdlNode::new("end_solve");
                node.push(("time", time.to_string()));
            }

            LogEvent::EndSession { time } => {
                node = KdlNode::new("end_session");
                node.push(("time", time.to_string()));
            }
        }
        node
    }
    fn from_kdl(node: &KdlNode) -> Option<Self> {
        match node.name().value() {
            "scramble" => Some(LogEvent::Scramble),

            "click" => {
                let mut layers = None;
                let mut target = None;
                let mut reverse = None;
                for entry in node.entries() {
                    let value = entry.value();
                    match entry.name().map(|name| name.value()) {
                        Some("layers") => {
                            layers = Some(LayerMask(u32::try_from(value.as_i64()?).ok()?));
                        }
                        Some("target") => target = value.as_string().map(str::to_owned),
                        Some("reverse") => reverse = value.as_bool(),
                        _ => (), // ignore unknown
                    }
                }
                Some(LogEvent::Click {
                    layers: layers.unwrap_or_default(),
                    target: target?,
                    reverse: reverse.unwrap_or(false),
                })
            }

            "twists" => Some(LogEvent::Twists(
                node.entries().first()?.value().as_string()?.to_owned(),
            )),

            "end_solve" => {
                let mut time = None;
                for entry in node.entries() {
                    match entry.name().map(|name| name.value()) {
                        Some("time") => {
                            time = Some(Timestamp::from_str(entry.value().as_string()?).ok()?)
                        }
                        _ => (), // ignore unknown
                    }
                }
                Some(LogEvent::EndSolve { time: time? })
            }

            "end_session" => {
                let mut time = None;
                for entry in node.entries() {
                    match entry.name().map(|name| name.value()) {
                        Some("time") => {
                            time = Some(Timestamp::from_str(entry.value().as_string()?).ok()?)
                        }
                        _ => (), // ignore unknown
                    }
                }
                Some(Self::EndSession { time: time? })
            }

            _ => None, // ignore unknown
        }
    }
}

fn set_children_to_events_list(node: &mut KdlNode, events: &[LogEvent]) {
    let mut children = KdlDocument::new();
    *children.nodes_mut() = events.iter().map(|ev| ev.to_kdl()).collect();
    node.set_children(children);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_puzzle_log_export() {
        println!(
            "{}",
            LogFile {
                program: Some(Program {
                    name: Some("Hyperspeedcube".to_string()),
                    version: Some("2.0.0-pre.15".to_string()),
                }),
                solves: vec![Solve {
                    puzzle: Puzzle {
                        id: "ft_cube:3".to_string(),
                        version: "1.0.0".to_string(),
                    },
                    solved: true,
                    duration: Some(5 * 60 * 1000),
                    scramble: Some(Scramble {
                        info: ScrambleInfo {
                            ty: ScrambleType::Partial(3),
                            time: Timestamp::now(),
                            seed: 42,
                        },
                        twists: "R U L'".to_string(),
                    }),
                    log: vec![
                        LogEvent::Scramble,
                        LogEvent::Twists("L U' R'".to_string()),
                        LogEvent::EndSolve {
                            time: Timestamp::now()
                        },
                        LogEvent::EndSession {
                            time: Timestamp::now()
                        },
                    ]
                }]
            }
            .serialize()
        );
    }
}

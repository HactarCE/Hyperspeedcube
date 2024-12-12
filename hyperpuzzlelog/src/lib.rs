//! Structures and serialization/deserialization for puzzle twist notation and
//! log files.

#![allow(missing_docs)]

use hyperpuzzle::LayerMask;
use kdl::*;

mod notation;

/// Log file version. This **MUST** be incremented whenever breaking changes are
/// made to the log file format.
pub const LOG_FILE_VERSION: i64 = 2;

/// Type used for UTC timestamps in log files.
pub type Timestamp = chrono::DateTime<chrono::Utc>;

/// Returns the UTC timestamp for the present moment, according to the system
/// clock.
pub fn now() -> Timestamp {
    chrono::Utc::now()
}

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
        node.push(*ty);
        node.push(("time", time.to_string()));
        node.push(("seed", *seed as i64));
        set_children_to_events_list(&mut node, &[LogEvent::Twists(twists.to_owned())]);
        node
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrambleInfo {
    pub ty: ScrambleType,
    pub time: Timestamp,
    pub seed: u32,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ScrambleType {
    Full,
    Partial(u32),
}
impl From<ScrambleType> for KdlValue {
    fn from(value: ScrambleType) -> Self {
        match value {
            ScrambleType::Full => "full".into(),
            ScrambleType::Partial(n) => (n as i64).into(),
        }
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
impl From<&LogEvent> for KdlNode {
    fn from(value: &LogEvent) -> Self {
        let mut node;
        match value {
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
}

fn set_children_to_events_list(node: &mut KdlNode, events: &[LogEvent]) {
    let mut children = KdlDocument::new();
    *children.nodes_mut() = events.iter().map(KdlNode::from).collect();
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
                            time: chrono::Utc::now(),
                            seed: 42,
                        },
                        twists: "R U L'".to_string(),
                    }),
                    log: vec![
                        LogEvent::Scramble,
                        LogEvent::Twists("L U' R'".to_string()),
                        LogEvent::EndSolve { time: now() },
                        LogEvent::EndSession { time: now() },
                    ]
                }]
            }
            .serialize()
        );
    }
}

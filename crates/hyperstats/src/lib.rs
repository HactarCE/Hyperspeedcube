//! Database of personal bests.

use std::collections::BTreeMap;
use std::str::FromStr;

use eyre::Result;
use hyperkdl::{NodeContentsSchema, Warning};
use hyperpuzzle_core::Timestamp;
use hyperpuzzle_core::verification::SolveVerification;
use kdl::{KdlDocument, KdlDocumentFormat, KdlError};

/// Saves the statistics file, overwriting the existing one.
pub fn save(stats: &StatsDb) -> Result<()> {
    let path = hyperpaths::stats_file()?;
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    std::fs::write(path, stats.serialize())?;
    Ok(())
}

/// Loads the statistics file, backing it up if there were any errors or
/// warnings on load.
pub fn load() -> StatsDb {
    // IIFE to mimic try_block
    (|| {
        let path = hyperpaths::stats_file().ok()?;
        let file_contents = std::fs::read_to_string(path).ok()?;
        match StatsDb::deserialize(&file_contents) {
            Ok((stats, warnings)) if warnings.is_empty() => Some(stats),
            Ok((stats, warnings)) => {
                for warning in warnings {
                    log::warn!("warning loading stats: {warning}");
                }
                hyperpaths::move_to_backup_file(path);
                Some(stats)
            }
            Err(e) => {
                log::error!("error loading stats: {e}");
                hyperpaths::move_to_backup_file(path);
                None
            }
        }
    })()
    .unwrap_or_default()
}

/// Database of solve statistics.
#[derive(Debug, Default, Clone)]
pub struct StatsDb(BTreeMap<String, PuzzlePBs>);
impl StatsDb {
    /// Serializes the PB database to a string.
    pub fn serialize(&self) -> String {
        let mut doc = KdlDocument::new();
        doc.set_format(KdlDocumentFormat {
            leading: "// Hyperspeedcube PB database\n".to_string(),
            trailing: String::new(),
        });

        for (puzzle_id, pbs) in &self.0 {
            doc.nodes_mut().push(pbs.to_kdl_node_with_name(puzzle_id));
        }

        doc.autoformat();

        doc.to_string()
    }
    /// Deserializes the PB database to a string.
    pub fn deserialize(s: &str) -> Result<(Self, Vec<Warning>), KdlError> {
        let doc = KdlDocument::from_str(s)?;

        let mut warnings = vec![];
        let mut ctx = hyperkdl::DeserCtx::new(&mut warnings);
        let pbs_iter = doc.nodes().iter().enumerate().filter_map(|(i, node)| {
            let ctx = ctx.with(hyperkdl::KeyPathElem::Child(i));
            let puzzle_id = node.name().value().to_owned();
            let pbs = PuzzlePBs::from_kdl_node_contents(node, ctx)?;
            Some((puzzle_id, pbs))
        });
        Ok((Self(pbs_iter.collect()), warnings))
    }

    /// Records a solve and updates the PB database.
    pub fn record_new_pb(&mut self, verification: &SolveVerification, filename: &str) {
        let new_pbs @ NewPbs {
            first,
            fmc,
            speed,
            blind,
        } = self.check_new_pb(verification);

        if !new_pbs.any() {
            return;
        }

        let pbs = self
            .0
            .entry(verification.puzzle_canonical_id.clone())
            .or_default();

        if first && let Some(time) = verification.timestamps.solve_completion {
            pbs.first = Some(FirstSolve {
                time: Timestamp(time),
            });
        }

        if fmc {
            pbs.fmc = Some(FmcPB {
                file: filename.to_string(),
                stm: verification.solution_stm.try_into().unwrap_or(i64::MAX),
            });
        }

        if speed && let Some(dur) = verification.durations.speedsolve {
            pbs.speed = Some(SpeedPB {
                file: filename.to_string(),
                duration: dur.num_milliseconds(),
            });
        }

        if blind && let Some(dur) = verification.durations.blindsolve {
            pbs.blind = Some(SpeedPB {
                file: filename.to_string(),
                duration: dur.num_milliseconds(),
            });
        }
    }

    /// Returns whether a solve breaks existing PBs.
    pub fn check_new_pb(&mut self, verification: &SolveVerification) -> NewPbs {
        let old_pbs = self
            .0
            .get(&verification.puzzle_canonical_id)
            .cloned()
            .unwrap_or_default();

        NewPbs {
            first: verification
                .timestamps
                .solve_completion
                .is_some_and(|time| old_pbs.first.is_none_or(|old_pb| old_pb.time.0 > time)),

            fmc: old_pbs.fmc.as_ref().is_none_or(|old_pb| {
                old_pb.stm > verification.solution_stm.try_into().unwrap_or(i64::MAX)
            }),

            speed: verification.durations.speedsolve.is_some_and(|dur| {
                old_pbs
                    .speed
                    .as_ref()
                    .is_none_or(|old_pb| old_pb.duration > dur.num_milliseconds())
            }),

            blind: verification.durations.blindsolve.is_some_and(|dur| {
                old_pbs
                    .blind
                    .as_ref()
                    .is_none_or(|old_pb| old_pb.duration > dur.num_milliseconds())
            }),
        }
    }
}

/// Categories in which a new solve is a personal best.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct NewPbs {
    /// First solution to a puzzle.
    pub first: bool,
    /// Shortest (fewest move count) solution to a puzzle.
    pub fmc: bool,
    /// Fastest speedsolve of a puzzle.
    pub speed: bool,
    /// Fastest blindsolve of a puzzle.
    pub blind: bool,
}
impl NewPbs {
    /// Returns whether the solve is a PB in any category.
    pub fn any(self) -> bool {
        let Self {
            first,
            fmc,
            speed,
            blind,
        } = self;
        first || fmc || speed || blind
    }
}

/// Personal bests for a single puzzle.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, hyperkdl_derive::NodeContents)]
pub struct PuzzlePBs {
    /// First solve.
    #[kdl(child("first"), optional)]
    pub first: Option<FirstSolve>,
    /// Fewest move count PB.
    #[kdl(child("fmc"), optional)]
    pub fmc: Option<FmcPB>,
    /// Speedsolving PB.
    #[kdl(child("speed"), optional)]
    pub speed: Option<SpeedPB>,
    /// Blindsolving PB.
    #[kdl(child("blind"), optional)]
    pub blind: Option<SpeedPB>,
}

/// First solve.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, hyperkdl_derive::NodeContents)]
pub struct FirstSolve {
    /// Timestamp when the puzzle was first solved.
    #[kdl(argument, proxy = hyperpuzzle_log::KdlProxy)]
    pub time: Timestamp,
}

/// Twist count PB.
#[derive(Debug, Clone, PartialEq, Eq, Hash, hyperkdl_derive::NodeContents)]
pub struct FmcPB {
    /// Log file path within the `solves` directory.
    #[kdl(property("file"))]
    pub file: String,
    /// Twist count in Slice Turn Metric.
    #[kdl(property("stm"))]
    pub stm: i64,
}

/// Speed PB.
#[derive(Debug, Clone, PartialEq, Eq, Hash, hyperkdl_derive::NodeContents)]
pub struct SpeedPB {
    /// Log file path within the `solves` directory.
    #[kdl(property("file"))]
    pub file: String,
    /// Duration in milliseconds.
    #[kdl(property("duration"))]
    pub duration: i64,
}

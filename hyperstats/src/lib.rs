//! Database of personal bests.

use std::collections::BTreeMap;
use std::str::FromStr;

use eyre::Result;
use hyperkdl::{NodeContentsSchema, Warning};
use hyperpuzzle_library::SolveVerification;
use kdl::{KdlDocument, KdlError};

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
        doc.set_leading("// Hyperspeedcube PB database\n");

        for (puzzle_id, pbs) in &self.0 {
            doc.nodes_mut().push(pbs.to_kdl_node_with_name(puzzle_id));
        }

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
        let new_pbs @ NewPbs { fmc, speed, blind } = self.check_new_pb(verification);

        if !new_pbs.any() {
            return;
        }

        let pbs = self.0.entry(verification.puzzle.id.clone()).or_default();

        if fmc {
            pbs.fmc = Some(FmcPB {
                file: filename.to_string(),
                stm: verification
                    .solution_stm_count
                    .try_into()
                    .unwrap_or(i64::MAX),
            });
        }

        if speed {
            if let Some(dur) = verification.speedsolve_duration {
                pbs.speed = Some(SpeedPB {
                    file: filename.to_string(),
                    duration: dur.num_milliseconds(),
                });
            }
        }

        if blind {
            if let Some(dur) = verification.blindsolve_duration {
                pbs.blind = Some(SpeedPB {
                    file: filename.to_string(),
                    duration: dur.num_milliseconds(),
                });
            }
        }
    }

    /// Returns whether a solve breaks existing PBs.
    pub fn check_new_pb(&mut self, verification: &SolveVerification) -> NewPbs {
        let old_pbs = self
            .0
            .get(&verification.puzzle.id)
            .cloned()
            .unwrap_or_default();

        NewPbs {
            fmc: old_pbs.fmc.as_ref().is_none_or(|old_pb| {
                old_pb.stm
                    > verification
                        .solution_stm_count
                        .try_into()
                        .unwrap_or(i64::MAX)
            }),

            speed: verification
                .speedsolve_duration
                .filter(|_| verification.single_session)
                .is_some_and(|dur| {
                    old_pbs
                        .speed
                        .as_ref()
                        .is_none_or(|old_pb| old_pb.duration > dur.num_milliseconds())
                }),

            blind: verification
                .blindsolve_duration
                .filter(|_| verification.single_session)
                .is_some_and(|dur| {
                    old_pbs
                        .blind
                        .as_ref()
                        .is_none_or(|old_pb| old_pb.duration > dur.num_milliseconds())
                }),
        }
    }
}

/// Categories in which a new solve is a personal best.
#[allow(missing_docs)]
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub struct NewPbs {
    pub fmc: bool,
    pub speed: bool,
    pub blind: bool,
}
impl NewPbs {
    /// Returns whether the solve is a PB in any category.
    pub fn any(self) -> bool {
        let Self { fmc, speed, blind } = self;
        fmc || speed || blind
    }
}

/// Personal bests for a single puzzle.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, hyperkdl_derive::NodeContents)]
pub struct PuzzlePBs {
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

/// Twist count PB.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, hyperkdl_derive::NodeContents)]
pub struct FmcPB {
    /// Log file path within the `solves` directory.
    #[kdl(property("file"))]
    pub file: String,
    /// Twist count in Slice Turn Metric.
    #[kdl(property("stm"))]
    pub stm: i64,
}

/// Speed PB.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, hyperkdl_derive::NodeContents)]
pub struct SpeedPB {
    /// Log file path within the `solves` directory.
    #[kdl(property("file"))]
    pub file: String,
    /// Duration in milliseconds.
    #[kdl(property("duration"))]
    pub duration: i64,
}

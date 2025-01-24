use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use eyre::Result;
use parking_lot::Mutex;

use super::{GeneratorParam, Progress, Redirectable};
use crate::{ColorSystem, Logger, Puzzle, TagSet, Version};

#[derive(Clone)]
pub struct BuildCtx {
    pub logger: Logger,
    pub progress: Arc<Mutex<Progress>>,
}
impl BuildCtx {
    pub(super) fn new(logger: &Logger, progress: &Arc<Mutex<Progress>>) -> Self {
        Self {
            logger: logger.clone(),
            progress: Arc::clone(progress),
        }
    }
}

pub struct PuzzleSpec {
    pub meta: PuzzleListMetadata,
    pub build: Box<dyn Send + Sync + Fn(BuildCtx) -> Result<Redirectable<Arc<Puzzle>>>>,
}
impl fmt::Debug for PuzzleSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PuzzleSpec")
            .field("meta", &self.meta)
            .finish()
    }
}

pub struct PuzzleSpecGenerator {
    pub meta: PuzzleListMetadata,
    pub params: Vec<GeneratorParam>,
    pub examples: HashMap<String, Arc<PuzzleSpec>>,
    pub generate:
        Box<dyn Send + Sync + Fn(BuildCtx, Vec<&str>) -> Result<Redirectable<Arc<PuzzleSpec>>>>,
}
impl fmt::Debug for PuzzleSpecGenerator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PuzzleSpec")
            .field("meta", &self.meta)
            .field("params", &self.params)
            .finish()
    }
}

pub struct ColorSystemGenerator {
    pub id: String,
    pub name: Option<String>,
    pub params: Vec<GeneratorParam>,
    pub generate:
        Box<dyn Send + Sync + Fn(BuildCtx, Vec<&str>) -> Result<Redirectable<Arc<ColorSystem>>>>,
}
impl fmt::Debug for ColorSystemGenerator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ColorSystemGenerator")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("params", &self.params)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct PuzzleListMetadata {
    /// Internal ID for the puzzle.
    pub id: String,
    /// Semantic version for the puzzle, in the form `[major, minor, patch]`.
    ///
    /// - Major version changes indicate that log files may be incompatible.
    /// - Minor version changes indicate that scrambles may be incompatible.
    /// - Patch versions indicate any other changes, including user-facing
    ///   changes.
    /// - Major version `0` allows any breaking changes.
    pub version: Version,
    /// Human-friendly name for the puzzle.
    pub name: String,
    /// Human-friendly aliases for the puzzle.
    pub aliases: Vec<String>,
    /// Set of tags and associated values.
    pub tags: TagSet,
}
impl PuzzleListMetadata {
    pub(super) fn filename(&self) -> Option<String> {
        self.tags.filename().map(str::to_owned)
    }
}

/// Compare by puzzle ID.
impl PartialEq for PuzzleListMetadata {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
/// Compare by puzzle ID.
impl Eq for PuzzleListMetadata {}
/// Compare by puzzle ID.
impl PartialOrd for PuzzleListMetadata {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
/// Compare by puzzle ID.
impl Ord for PuzzleListMetadata {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        crate::compare_ids(&self.id, &other.id)
    }
}

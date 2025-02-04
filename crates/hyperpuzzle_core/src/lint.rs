use std::sync::Arc;

use crate::PuzzleSpec;

/// List of lint warnings for a puzzle specification.
#[derive(Debug, Clone)]
pub struct PuzzleLintOutput {
    /// Puzzle specification.
    pub puzzle: Arc<PuzzleSpec>,
    /// Puzzle schema version.
    pub schema: u64,
    /// Tags that should be present or explicitly excluded but aren't.
    pub missing_tags: Vec<&'static Vec<Arc<str>>>,
}

impl PuzzleLintOutput {
    /// Lints a puzzle specification.
    pub fn from_spec(puzzle: &Arc<PuzzleSpec>) -> Self {
        let tags = &puzzle.meta.tags;
        let schema = tags.get("schema").and_then(|v| v.as_int()).unwrap_or(0) as u64;
        let expected_tag_sets = crate::TAGS
            .expected_tag_sets()
            .iter()
            .filter(|(&k, _)| k > schema)
            .flat_map(|(_, v)| v);
        let missing_tags = expected_tag_sets
            .filter(|&tag_set| tag_set.iter().all(|tag| !tags.0.contains_key(&**tag)))
            .collect();

        Self {
            puzzle: Arc::clone(puzzle),
            schema,
            missing_tags,
        }
    }

    /// Returns `true` if there are no issues with the puzzle specification.
    pub fn all_good(&self) -> bool {
        let Self {
            puzzle: _,
            schema,
            missing_tags,
        } = self;
        *schema == crate::TAGS.schema && missing_tags.is_empty()
    }
}

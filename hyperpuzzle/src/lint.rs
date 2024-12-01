use std::sync::Arc;

use crate::lua::PuzzleSpec;

/// List of lint warnings for a puzzle specification.
#[derive(Debug, Clone)]
pub struct PuzzleLintOutput {
    /// Puzzle specification.
    pub puzzle: Arc<PuzzleSpec>,
    /// Tags that should be present or explicitly excluded but aren't.
    pub missing_tags: Vec<&'static Vec<Arc<str>>>,
}

impl PuzzleLintOutput {
    /// Lints a puzzle specification.
    pub fn from_spec(puzzle: Arc<PuzzleSpec>) -> Self {
        let missing_tags = crate::TAGS
            .expected_tag_sets()
            .iter()
            .filter(|&tag_set| {
                tag_set
                    .iter()
                    .all(|tag| !puzzle.tags.0.contains_key(&**tag))
            })
            .collect();

        Self {
            puzzle,
            missing_tags,
        }
    }

    /// Returns `true` if there are no issues with the puazle specification.
    pub fn all_good(&self) -> bool {
        let Self {
            puzzle: _,
            missing_tags,
        } = self;
        missing_tags.is_empty()
    }
}

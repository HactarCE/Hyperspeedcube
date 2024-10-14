use std::sync::Arc;

use crate::lua::PuzzleSpec;

#[derive(Debug, Clone)]
pub struct PuzzleLintOutput {
    pub puzzle: Arc<PuzzleSpec>,
    pub missing_tags: Vec<&'static Vec<Arc<str>>>,
}

impl PuzzleLintOutput {
    pub fn from_spec(puzzle: Arc<PuzzleSpec>) -> Self {
        let missing_tags = crate::TAGS
            .expected_tag_sets()
            .iter()
            .filter(|&tag_set| tag_set.iter().all(|tag| !puzzle.tags.contains_key(&**tag)))
            .collect();

        Self {
            puzzle,
            missing_tags,
        }
    }

    pub fn all_good(&self) -> bool {
        let Self {
            puzzle: _,
            missing_tags,
        } = self;
        missing_tags.is_empty()
    }
}

use std::sync::Arc;

use crate::CatalogMetadata;

/// List of lint warnings for a puzzle specification.
#[derive(Debug, Clone)]
pub struct PuzzleLintOutput {
    /// Puzzle metadata.
    pub meta: Arc<CatalogMetadata>,
    /// Puzzle schema version.
    pub schema: u64,
    /// Tags that should be present or explicitly excluded but aren't.
    pub missing_tags: Vec<&'static Vec<Arc<str>>>,
}

impl PuzzleLintOutput {
    /// Lints a puzzle specification.
    pub fn from_meta(meta: &Arc<CatalogMetadata>) -> Self {
        let tags = &meta.tags;
        let schema = tags.get("schema").and_then(|v| v.as_int()).unwrap_or(0) as u64;
        let expected_tag_sets = crate::TAGS
            .expected_tag_sets()
            .iter()
            .filter(|&(&k, _)| k > schema)
            .flat_map(|(_, v)| v);
        let missing_tags = expected_tag_sets
            .filter(|&tag_set| tag_set.iter().all(|tag| !tags.0.contains_key(&**tag)))
            .collect();

        Self {
            meta: Arc::clone(meta),
            schema,
            missing_tags,
        }
    }

    /// Returns `true` if there are no issues with the puzzle specification.
    pub fn all_good(&self) -> bool {
        let Self {
            meta: _,
            schema,
            missing_tags,
        } = self;
        *schema == crate::TAGS.schema && missing_tags.is_empty()
    }
}

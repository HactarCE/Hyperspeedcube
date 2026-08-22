use super::*;

/// Puzzle list entry, which generally corresponds to a puzzle or puzzle
/// generator.
#[derive(Serialize, Debug, Clone)]
pub struct PuzzleListEntry {
    /// Catalog ID.
    pub id: CatalogId,
    /// Semantic version.
    pub version: Option<Version>, // TODO: should be required!
    /// Human-friendly name.
    pub name: String,
    /// Human-friendly aliases.
    pub aliases: Vec<String>,
    /// Set of tags and associated values.
    pub tags: TagSet,
}

/// Compare by catalog ID.
impl PartialEq for PuzzleListEntry {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

/// Compare by catalog ID.
impl Eq for PuzzleListEntry {}

/// Compare by catalog ID.
impl PartialOrd for PuzzleListEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Compare by catalog ID.
impl Ord for PuzzleListEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        CatalogId::cmp(&self.id, &other.id)
    }
}

impl PuzzleListEntry {
    /// Constructs metadata for an object with no version, aliases, or tags.
    pub fn simple(id: CatalogId, name: String) -> Self {
        Self {
            id,
            version: None,
            name,
            aliases: vec![],
            tags: TagSet::new(),
        }
    }

    /// Returns the equivalent CLI type.
    pub fn to_cli(&self) -> hyperspeedcube_cli_types::puzzle_info::PuzzleListEntry {
        hyperspeedcube_cli_types::puzzle_info::PuzzleListEntry {
            id: self.id.clone(),
            version: self.version.map(|v| [v.major, v.minor, v.patch]),
            name: self.name.clone(),
            aliases: self.aliases.clone(),
            tags: self.tags.to_cli(),
        }
    }
}

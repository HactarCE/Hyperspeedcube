use super::*;

/// Metadata for an object or object generator in a catalog.
#[derive(Serialize, Debug, Clone)]
pub struct CatalogMetadata {
    /// Internal ID.
    pub id: CatalogId,
    /// Semantic version.
    pub version: Version,
    /// Human-friendly name.
    pub name: String,
    /// Human-friendly aliases.
    pub aliases: Vec<String>,
    /// Set of tags and associated values.
    pub tags: TagSet,
}

/// Compare by catalog ID.
impl PartialEq for CatalogMetadata {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

/// Compare by catalog ID.
impl Eq for CatalogMetadata {}

/// Compare by catalog ID.
impl PartialOrd for CatalogMetadata {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Compare by catalog ID.
impl Ord for CatalogMetadata {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        CatalogId::cmp(&self.id, &other.id)
    }
}

impl CatalogMetadata {
    /// Constructs metadata for an object with no version, aliases, or tags.
    pub fn simple(id: CatalogId, name: String) -> Self {
        Self {
            id,
            version: Version::PLACEHOLDER,
            name,
            aliases: vec![],
            tags: TagSet::new(),
        }
    }

    /// Returns dummy metadata.
    pub fn dummy() -> Self {
        let id = CatalogId {
            base: "dummy".into(),
            args: vec![],
        };
        let name = String::new();
        Self::simple(id, name)
    }

    /// Returns the equivalent CLI type.
    pub fn to_cli(&self) -> hyperspeedcube_cli_types::puzzle_info::PuzzleListMetadata {
        hyperspeedcube_cli_types::puzzle_info::PuzzleListMetadata {
            id: self.id.clone(),
            version: [self.version.major, self.version.minor, self.version.patch],
            name: self.name.clone(),
            aliases: self.aliases.clone(),
            tags: self.tags.to_cli(),
        }
    }
}

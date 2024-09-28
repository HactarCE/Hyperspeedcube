/// Puzzle metadata.
#[derive(Debug, Default, Clone)]
pub struct PuzzleMetadata {
    /// Names of authors of the puzzle definition.
    pub authors: Vec<String>,
    /// Names of inventors of the original puzzle.
    pub inventors: Vec<String>,
    /// Other names this puzzle is known by.
    pub aliases: Vec<String>,
    /// External links for this puzzle.
    pub external: PuzzleMetadataExternal,
}

/// External links for a puzzle.
#[derive(Debug, Default, Clone)]
pub struct PuzzleMetadataExternal {
    /// WCA puzzle ID.
    pub wca: Option<String>,
}
impl PuzzleMetadataExternal {
    /// Returns the URL of the puzzle's WCA page.
    pub fn wca_url(&self) -> Option<String> {
        Some(format!(
            "https://www.worldcubeassociation.org/results/rankings/{}/single",
            self.wca.as_ref()?,
        ))
    }
}

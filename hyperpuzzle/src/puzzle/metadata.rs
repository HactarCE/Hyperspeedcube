#[derive(Debug, Default, Clone)]
pub struct PuzzleMetadata {
    pub authors: Vec<String>,
    pub inventors: Vec<String>,
    pub aliases: Vec<String>,
    pub external: PuzzleMetadataExternal,
}

#[derive(Debug, Default, Clone)]
pub struct PuzzleMetadataExternal {
    /// WCA puzzle ID.
    pub wca: Option<String>,
}

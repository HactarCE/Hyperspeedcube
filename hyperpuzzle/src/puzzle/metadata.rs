/// Puzzle metadata.
///
/// TODO: deprecate this
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

    pub properties: PuzzleProperties,

    /// Whether the puzzle is made from a puzzle generator.
    pub generated: bool,

    pub canonical: bool,

    pub meme: bool,
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

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct PuzzleProperties {
    pub puzzle_engine: Option<WhichPuzzleEngine>,
    pub rank: u8,
    pub ndim: u8,

    pub shape_category: Option<String>,
    pub shape: Option<String>,

    pub turning_elements: PuzzleTurningElements,

    pub axis_systems: Vec<String>,

    pub turning_properties: PuzzleTurningProperties,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum WhichPuzzleEngine {
    Euclidean,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct PuzzleCutProperties {
    pub depths: PuzzleCutDepths,
    pub piece_types: PuzzlePieceTypes,
}

bitflags! {
    #[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
    pub struct PuzzleCutDepths: u8 {
        const SHALLOW = 1;
        const ADJACENT = 1 << 1;
        const DEEPER_THAN_ADJACENT = 1 << 2;
        const HALF = 1 << 3;
        const DEEPER_THAN_ORIGIN = 1 << 4;
    }

    #[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
    pub struct PuzzlePieceTypes: u8 {
        const ACRONIC = 1;
        const SPHENIC = 1 << 1;
        const STANDARD = 1 << 2;
        const TRIVIAL = 1 << 3;
    }

    #[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
    pub struct PuzzleTurningElements: u8 {
        const FACET = 1;
        const RIDGE = 1 << 1;
        const EDGE = 1 << 2;
        const VERTEX = 1 << 3;
        const OTHER = 1 << 4;
    }

    #[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
    pub struct PuzzleTurningProperties: u8 {
        const DOCTRINAIRE = 1;
        const JUMBLING = 1 << 1;
        const SHAPESHIFTING = 1 << 2;
        const REDUCED = 1 << 3;
        const TWISTING = 1 << 4;
        const SLIDING = 1 << 5;
    }
}

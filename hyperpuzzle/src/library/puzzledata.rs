/// Puzzle metadata known before building the puzzle.
#[derive(Debug, Clone)]
pub struct PuzzleData {
    /// ID of the puzzle.
    pub id: String,
    /// Name of the puzzle.
    pub name: String,
    /// Name of the file containing the puzzle definition.
    pub filename: String,
}

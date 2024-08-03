use hyperpuzzle::*;

#[derive(Debug, Clone)]
pub struct PuzzleFiltersState {
    pub colors: PerColor<Option<bool>>,
    pub piece_types: PerPieceType<Option<bool>>,
}
impl PuzzleFiltersState {
    pub fn new(puzzle: &Puzzle) -> Self {
        Self {
            colors: puzzle.colors.list.map_ref(|_, _| None),
            piece_types: puzzle.piece_types.map_ref(|_, _| None),
        }
    }
}

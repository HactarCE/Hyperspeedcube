use std::fmt;
use std::sync::{Arc, Weak};

use super::{Mesh, PieceInfo, PieceTypeInfo, PuzzleState, StickerInfo};
use crate::{PerPiece, PerPieceType, PerSticker};

/// Puzzle type info.
pub struct Puzzle {
    /// Reference-counted pointer to this struct.
    pub this: Weak<Puzzle>,
    /// Human-friendly name for the puzzle.
    pub name: String,
    /// Internal ID for the puzzle.
    pub id: String,

    /// Puzzle mesh for rendering.
    pub mesh: Mesh,

    /// List of pieces, indexed by ID.
    pub pieces: PerPiece<PieceInfo>,
    /// List of stickers, indexed by ID.
    pub stickers: PerSticker<StickerInfo>,
    /// List of piece types, indexed by ID.
    pub piece_types: PerPieceType<PieceTypeInfo>,

    /// Number of moves for a full scramble.
    pub scramble_moves_count: usize,

    /// Move notation.
    pub notation: NotationScheme,

    /// Function to create a new solved puzzle state.
    pub new: Box<dyn Send + Sync + Fn(Arc<Puzzle>) -> PuzzleState>,
}
impl fmt::Debug for Puzzle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Puzzle")
            .field("this", &self.this)
            .field("name", &self.name)
            .field("id", &self.id)
            .field("mesh", &self.mesh)
            .field("pieces", &self.pieces)
            .field("stickers", &self.stickers)
            .field("piece_types", &self.piece_types)
            .field("scramble_moves_count", &self.scramble_moves_count)
            .field("notation", &self.notation)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct PuzzleTwists {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct NotationScheme {}

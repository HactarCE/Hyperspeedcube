use std::sync::{Arc, Weak};

use super::{Mesh, PieceInfo, PieceTypeInfo, PuzzleState, StickerInfo};
use crate::{PerPiece, PerPieceType, PerSticker};

/// Puzzle type info.
pub struct Puzzle {
    /// Reference-counted pointer to this struct.
    pub this: Weak<Puzzle>,
    /// Human-friendly name of the puzzle.
    pub name: String,
    /// Base shape, without any internal cuts.
    pub shape: Arc<PuzzleShape>,
    /// Twist set.
    pub twists: Arc<PuzzleTwists>,

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

pub struct PuzzleShape {
    pub name: String,
    pub ndim: u8,
}

pub struct PuzzleTwists {
    pub name: String,
}

pub struct NotationScheme {}

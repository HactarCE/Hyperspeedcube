use std::sync::{Arc, Weak};

use crate::{AxisInfo, PerAxis};

use super::{
    Axis, ColorInfo, LayerMask, Mesh, Notation, PerColor, PerPiece, PerPieceType, PerSticker,
    PieceInfo, PieceTypeInfo, PuzzleState, StickerInfo, Twist,
};
use hypershape::prelude::*;

/// Puzzle type info.
#[derive(Debug)]
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
    /// List of colors, indexed by ID.
    pub colors: PerColor<ColorInfo>,

    /// Number of moves for a full scramble.
    pub scramble_moves_count: usize,

    /// Move notation.
    pub notation: Notation,

    /// List of axes, indexed by ID.
    pub axes: PerAxis<AxisInfo>,

    /// Space containing a polytope for each piece.
    pub(crate) space: Space,
    /// Polytope for each piece.
    pub(crate) piece_polytopes: PerPiece<AtomicPolytopeRef>,
    /// Manifold for each axis, for each layer.
    pub(crate) axis_manifolds: PerAxis<Vec<ManifoldRef>>,
}

impl Puzzle {
    /// Returns an `Arc` reference to the puzzle type.
    pub fn arc(&self) -> Arc<Self> {
        self.this.upgrade().expect("`Puzzle` removed from `Arc`")
    }
    /// Constructs a new instance of the puzzle.
    pub fn new_solved_state(&self) -> PuzzleState {
        PuzzleState::new(self.arc())
    }

    /// Returns the number of dimensions of the puzzle.
    pub fn ndim(&self) -> u8 {
        self.mesh.ndim()
    }

    pub(crate) fn opposite_twist_axis(&self, _axis: Axis) -> Option<Axis> {
        todo!()
    }
    pub(crate) fn axis_of(&self, _twist: Twist) -> Axis {
        todo!()
    }
    pub fn layer_count(&self) -> u8 {
        todo!()
    }
    pub(crate) fn all_layers(&self) -> LayerMask {
        todo!()
    }
    pub(crate) fn count_quarter_turns(&self, _twist: Twist) -> usize {
        todo!()
    }
    pub fn twist_direction_from_name(&self, _direction: &str) -> Option<String> {
        todo!()
    }
    pub fn twist_command_short_description(
        &self,
        _axis_name: Option<String>,
        _direction_name: String,
        _layers: LayerMask,
    ) -> String {
        todo!()
    }
    pub fn twist_axis_from_name(&self, _axis_name: &str) -> Option<String> {
        todo!()
    }
    pub fn make_recenter_twist(&self, _twist_axis: String) -> Result<(Twist, LayerMask), ()> {
        todo!()
    }
}

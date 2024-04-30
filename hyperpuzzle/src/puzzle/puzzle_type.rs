use std::collections::HashMap;
use std::sync::{Arc, Weak};

use hypermath::pga::Motor;
use hypermath::Float;
use hypershape::Space;
use parking_lot::Mutex;

use super::*;

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
    /// Map from axis name to axis.
    pub axis_by_name: HashMap<String, Axis>,

    /// List of twists, indexed by ID.
    pub twists: PerTwist<TwistInfo>,
    /// Map from twist name to twist.
    pub twist_by_name: HashMap<String, Twist>,

    /// Space containing a polytope for each piece.
    pub(crate) space: Mutex<Space>,
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

    /// Returns the transform to apply to pieces during an animation.
    ///
    /// `t` ranges from `0.0` to `1.0`.
    pub fn partial_twist_transform(&self, twist: Twist, t: Float) -> Motor {
        let identity = Motor::ident(self.ndim());
        let twist_transform = &self.twists[twist].transform;
        Motor::slerp_infallible(&identity, twist_transform, t)
    }

    pub(crate) fn opposite_twist_axis(&self, _axis: Axis) -> Option<Axis> {
        todo!()
    }
    pub(crate) fn axis_of(&self, _twist: Twist) -> Axis {
        todo!()
    }
    pub(crate) fn all_layers(&self) -> LayerMask {
        todo!()
    }
    pub(crate) fn count_quarter_turns(&self, _twist: Twist) -> usize {
        todo!()
    }
    /// Returns a twist that recenters the given twist axis.
    pub fn make_recenter_twist(&self, _twist_axis: String) -> Result<(Twist, LayerMask), ()> {
        todo!()
    }
}

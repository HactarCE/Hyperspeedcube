use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, Weak};

use hypershape::Space;
use indexmap::IndexMap;

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

    /// Space containing a polytope for each piece.
    pub(crate) space: Arc<Space>,
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

    /// List of named color schemes.
    pub color_schemes: IndexMap<String, PerColor<Option<DefaultColor>>>,
    /// Name of the default color scheme, which is typically `"Default"`.
    pub default_color_scheme: String,

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

    /// Twist for each face of a twist gizmo.
    pub gizmo_twists: PerGizmoFace<Twist>,
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
        self.mesh.ndim
    }

    /// Returns the default color scheme for the puzzle.
    pub fn default_color_scheme(&self) -> Cow<'_, PerColor<Option<DefaultColor>>> {
        match self.color_schemes.get(&self.default_color_scheme) {
            Some(scheme) => Cow::Borrowed(scheme),
            None => {
                let mut ret = PerColor::new();
                ret.resize(self.colors.len()).expect("impossible overflow!");
                Cow::Owned(ret)
            }
        }
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

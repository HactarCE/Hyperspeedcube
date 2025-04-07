use std::any::Any;
use std::fmt;
use std::sync::Arc;

use hypermath::WhichSide;

use super::*;
use crate::{Axis, LayerMask, LayeredTwist, PerPiece, Piece, PieceMask, Puzzle};

/// Instance of a puzzle with a particular state.
///
/// In order to be dyn-compatible, this trait has no associated types and uses
/// wrappers around `Box<dyn Trait>` instead.
pub trait PuzzleState: 'static + fmt::Debug + Any + Send + Sync {
    /// Returns the puzzle type.
    fn ty(&self) -> &Arc<Puzzle>;

    /// Clones the puzzle.
    fn clone_dyn(&self) -> BoxDynPuzzleState;

    /*
     * PUZZLE LOGIC
     */

    /// Applies a twist and returns the new puzzle state or an error containing
    /// the set of pieces that prevented the twist.
    fn do_twist(&self, twist: LayeredTwist) -> Result<Self, Vec<Piece>>
    where
        Self: Sized;

    /// Applies a twist and returns the new puzzle state or an error containing
    /// the set of pieces that prevented the twist.
    fn do_twist_dyn(&self, twist: LayeredTwist) -> Result<BoxDynPuzzleState, Vec<Piece>>;

    /// Returns whether the puzzle is in a solved state.
    fn is_solved(&self) -> bool;

    /*
     * GRIPS
     */

    /// Returns each piece's location with respect to a grip (axis + layer
    /// mask). A piece may be inside the grip, outside the grip, or blocking the
    /// grip. [`WhichSide::Flush`] is not used.
    fn compute_grip(&self, axis: Axis, layers: LayerMask) -> PerPiece<WhichSide>;

    /// Returns the set of pieces on the inside of a grip (axis + layer mask).
    /// This considers blocking pieces to be outside the grip; use
    /// [`PuzzleState::compute_grip()`] to see which pieces are blocking a
    /// twist.
    fn compute_gripped_pieces(&self, axis: Axis, layers: LayerMask) -> PieceMask {
        PieceMask::from_iter(
            self.ty().pieces.len(),
            self.compute_grip(axis, layers)
                .iter_filter(|_, &status| status == WhichSide::Inside),
        )
    }

    /// Returns the smallest layer mask on `axis` that contains `piece`, or
    /// `None` if none exists.
    fn min_layer_mask(&self, axis: Axis, piece: Piece) -> Option<LayerMask>;

    /// Returns the smallest unblocked layer mask on `axis` that contains
    /// `piece`, or `None` if none exists.
    fn min_drag_layer_mask(&self, axis: Axis, piece: Piece) -> Option<LayerMask>;

    /*
     * RENDERING
     */

    /// Returns data to render the current state of the puzzle.
    fn render_data(&self) -> BoxDynPuzzleStateRenderData;

    /// Returns data to render the state of the puzzle during a twist animation.
    ///
    /// `t` ranges from 0 to 1. Motion should be perceptually linear with
    /// respect to `t`.
    fn partial_twist_render_data(&self, twist: LayeredTwist, t: f32)
    -> BoxDynPuzzleStateRenderData;

    /// Returns data to render the state of the puzzle during an animation.
    ///
    /// `t` ranges from 0 to 1. Motion should be perceptually linear with
    /// respect to `t`.
    ///
    /// # Panics
    ///
    /// This method may panics if passed an invalid animation.
    fn animated_render_data(
        &self,
        anim: &BoxDynPuzzleAnimation,
        t: f32,
    ) -> BoxDynPuzzleStateRenderData;
}

box_dyn_wrapper_struct! {
    /// Wrapper around `Box<dyn PuzzleState>` that can be downcast to a concrete
    /// puzzle state type. This type also implements [`Clone`] for convenience.
    pub struct BoxDynPuzzleState(Box<dyn PuzzleState>);
}
impl_dyn_clone!(for BoxDynPuzzleState);

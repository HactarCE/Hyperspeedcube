use std::any::Any;
use std::fmt;
use std::sync::Arc;

use hypermath::WhichSide;

use crate::{Axis, LayerMask, LayeredTwist, PerPiece, Piece, PieceMask, Puzzle};

/// Instance of a puzzle with a particular state.
///
/// In order to be dyn-compatible, this trait has no associated types. Instead
/// it uses `Box<dyn Any>` for rendering data. All implementors of this trait
/// should explicitly document which type(s) may be returned from
/// [`PuzzleState::render_data()`].
pub trait PuzzleState: 'static + fmt::Debug + Clone + Send + Sync {
    /// Returns the puzzle type.
    fn ty(&self) -> &Arc<Puzzle>;

    /*
     * PUZZLE LOGIC
     */

    /// Does a twist, or returns an error containing the set of pieces that
    /// prevented the twist.
    fn do_twist(&self, twist: LayeredTwist) -> Result<Self, Vec<Piece>>;

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

    /// Returns data to render the state of the puzzle during an animation.
    ///
    /// `t` ranges from 0 to 1. Motion should be perceptually linear with
    /// respect to `t`.
    ///
    /// # Panics
    ///
    /// This method may panics if passed an invalid animation.
    fn render_data_with_animation(
        &self,
        anim: &BoxDynPuzzleAnimation,
        t: f32,
    ) -> BoxDynPuzzleStateRenderData;
}

macro_rules! box_dyn_wrapper_struct {
    {
        $(#[$attr:meta])*
        $vis:vis struct $struct_name:ident(Box<dyn $trait_name:ident>);
     } => {
        $(#[$attr])*
        $vis struct $struct_name(Box<dyn $trait_name>);
        impl fmt::Debug for $struct_name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_tuple(stringify!($struct_name))
                    .finish_non_exhaustive()
            }
        }
        impl<T: $trait_name> From<T> for $struct_name {
            fn from(value: T) -> Self {
                Self(Box::new(value))
            }
        }
        impl $struct_name {
            /// Attempts to downcast to a concrete type.
            pub fn downcast<T: $trait_name>(self) -> Option<Box<T>> {
                (self.0 as Box<dyn Any>).downcast().ok()
            }
            /// Attempts to downcast a reference to a concrete type.
            pub fn downcast_ref<T: $trait_name>(&self) -> Option<&T> {
                (&*self.0 as &dyn Any).downcast_ref()
            }
        }
    };
}

/// Marker trait for types that may be returned from
/// [`PuzzleState::render_data()`].
///
/// Because [`Any`] is defined with a `'static` bound, implementors of this
/// trait cannot borrow from the puzzle state.
pub trait PuzzleStateRenderData: Any + Send + Sync {}
box_dyn_wrapper_struct! {
    /// Wrapper around `Box<dyn PuzzleStateRenderData>` that can be downcast to
    /// a concrete render data type.
    pub struct BoxDynPuzzleStateRenderData(Box<dyn PuzzleStateRenderData>);
}

/// Marker trait for types that may be used as animations.
pub trait PuzzleAnimation: Any + Send + Sync {
    /// Returns a copy of the data.
    fn dyn_clone(&self) -> BoxDynPuzzleAnimation;
}
box_dyn_wrapper_struct! {
    /// Wrapper around `Box<dyn PuzzleAnimation>` that can be downcast to a
    /// concrete animation type. This type also implements [`Clone`] for
    /// conveninence.
    pub struct BoxDynPuzzleAnimation(Box<dyn PuzzleAnimation>);
}
impl Clone for BoxDynPuzzleAnimation {
    fn clone(&self) -> Self {
        self.0.dyn_clone()
    }
}

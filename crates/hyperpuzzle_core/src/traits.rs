use std::any::Any;
use std::fmt;
use std::sync::Arc;

use crate::Puzzle;

/// Instance of a puzzle with a particular state.
///
/// In order to be dyn-compatible, this trait has no associated types. Instead
/// it uses `Box<dyn Any>` for rendering data. All implementors of this trait
/// should explicitly document which type(s) may be returned from
/// [`PuzzleState::render_data()`].
pub trait PuzzleState: 'static + fmt::Debug + Clone + Send + Sync {
    /// Returns the puzzle type.
    fn ty(&self) -> &Arc<Puzzle>;

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
    ($vis:vis struct $struct_name:ident(Box<dyn $trait_name:ident>)) => {
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
            pub fn downcast<T: $trait_name>(self) -> Option<Box<T>> {
                (self.0 as Box<dyn Any>).downcast().ok()
            }
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
box_dyn_wrapper_struct!(pub struct BoxDynPuzzleStateRenderData(Box<dyn PuzzleStateRenderData>));

/// Marker trait for types that may be used as animations.
pub trait PuzzleAnimation: Any + Send + Sync {
    fn dyn_clone(&self) -> BoxDynPuzzleAnimation;
}
box_dyn_wrapper_struct!(pub struct BoxDynPuzzleAnimation(Box<dyn PuzzleAnimation>));
impl Clone for BoxDynPuzzleAnimation {
    fn clone(&self) -> Self {
        self.0.dyn_clone()
    }
}

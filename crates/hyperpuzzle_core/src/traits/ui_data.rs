use std::any::Any;
use std::fmt;

/// Marker trait for types that may be stored in [`Puzzle::ui_data`].
///
/// Because [`Any`] is defined with a `'static` bound, implementors of this
/// trait cannot borrow from the puzzle state.
pub trait PuzzleUiData: Any + Send + Sync {}
box_dyn_wrapper_struct! {
    /// Wrapper around `Box<dyn PuzzleTypeGpuData>` that can be downcast to a
    /// concrete GPU data type.
    pub struct BoxDynPuzzleUiData(Box<dyn PuzzleUiData>);
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
    fn clone_dyn(&self) -> BoxDynPuzzleAnimation;
}
box_dyn_wrapper_struct! {
    /// Wrapper around `Box<dyn PuzzleAnimation>` that can be downcast to a
    /// concrete animation type. This type also implements [`Clone`] for
    /// conveninence.
    pub struct BoxDynPuzzleAnimation(Box<dyn PuzzleAnimation>);
}
impl_dyn_clone!(for BoxDynPuzzleAnimation);

use std::any::Any;
use std::fmt;

use hyperpuzzle_core::{box_dyn_wrapper_struct, impl_dyn_clone};

// box_dyn_wrapper_struct! {
//     /// Wrapper around `Box<dyn PuzzleState>` that can be downcast to a concrete
//     /// puzzle state type.
//     pub struct BoxDynPuzzleEngine(Box<dyn PuzzleEngine>);
// }

// pub trait PuzzleEngine {
//     /// Name of the puzzle engine hook.
//     const ENGINE_NAME: &'static str;
//     /// Human-friendly string to display in type errors to represent the type.
//     const TYPE_NAME: &'static str;

//     /// Constructs a new puzzle builder from named arguments.
//     fn new(kwargs: IndexMap<Key, Value>) -> Result<BoxDynPuzzleEngine>;
//     /// Builds the puzzle.
//     fn build(self) -> Result<Puzzle>;
// }

box_dyn_wrapper_struct! {
    /// Wrapper around `Box<dyn CustomValue>` that can be downcast to a concrete
    /// puzzle state type. It also implements `Clone` for convenience.
    pub struct BoxDynValue(Box<dyn CustomValue>);
}
impl_dyn_clone!(for BoxDynValue);

pub trait CustomValue: Any + Send + Sync {
    fn type_name(&self) -> &'static str;
    fn clone_dyn(&self) -> BoxDynValue;
    fn fmt(&self, f: &mut fmt::Formatter<'_>, is_debug: bool) -> fmt::Result;
}

macro_rules! box_dyn_wrapper_struct {
    {
        $(#[$attr:meta])*
        $vis:vis struct $struct_name:ident(Box<dyn $trait_name:ident>);
    } => {
        $(#[$attr])*
        $vis struct $struct_name(Box<dyn $trait_name>);
        impl std::fmt::Debug for $struct_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
            /// Constructs a new boxed dynamic value.
            pub fn new<T: $trait_name>(value: T) -> Self {
                Self(Box::new(value))
            }

            /// Attempts to downcast to a concrete type.
            pub fn downcast<T: $trait_name>(self) -> Option<Box<T>> {
                (self.0 as Box<dyn Any>).downcast().ok()
            }
            /// Attempts to downcast a reference to a concrete type.
            pub fn downcast_ref<T: $trait_name>(&self) -> Option<&T> {
                (&*self.0 as &dyn Any).downcast_ref()
            }
        }
        impl std::ops::Deref for $struct_name {
            type Target = dyn $trait_name;

            fn deref(&self) -> &Self::Target {
                &*self.0
            }
        }
    };
}

macro_rules! impl_dyn_clone {
    (for $struct_name:ident) => {
        impl Clone for $struct_name {
            fn clone(&self) -> Self {
                self.0.clone_dyn()
            }
        }
    };
}

mod engine_data;
mod state;
mod ui_data;
mod vantage_set_data;
mod vantages;

pub use engine_data::{BoxDynTwistSystemEngineData, TwistSystemEngineData};
pub use state::{BoxDynPuzzleState, PuzzleState};
pub use ui_data::{
    BoxDynPuzzleAnimation, BoxDynPuzzleStateRenderData, BoxDynPuzzleUiData, PuzzleAnimation,
    PuzzleStateRenderData, PuzzleUiData,
};
pub use vantage_set_data::{BoxDynVantageSetEngineData, VantageSetEngineData};
pub use vantages::{
    BoxDynRelativeAxis, BoxDynRelativeTwist, BoxDynVantageGroup, BoxDynVantageGroupElement,
    RelativeAxis, RelativeTwist, SimpleRelativeAxis, SimpleRelativeTwist, SimpleVantageGroup,
    VantageGroup, VantageGroupElement,
};

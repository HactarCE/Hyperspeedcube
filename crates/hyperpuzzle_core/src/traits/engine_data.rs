use std::any::Any;

/// Marker trait for types that may be stored in
/// [`crate::TwistSystem::engine_data`].
pub trait TwistSystemEngineData: Any + Send + Sync {}
box_dyn_wrapper_struct! {
    /// Wrapper around `Box<dyn TwistSystemEngineData>` that can be downcast to
    /// a concrete data type.
    pub struct BoxDynTwistSystemEngineData(Box<dyn TwistSystemEngineData>);
}
impl TwistSystemEngineData for () {}

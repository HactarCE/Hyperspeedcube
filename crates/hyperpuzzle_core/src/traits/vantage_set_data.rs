use std::any::Any;

/// Marker trait for types that may be stored in [`crate::VantageSet::engine_data`].
pub trait VantageSetEngineData: Any + Send + Sync {}
box_dyn_wrapper_struct! {
    /// Wrapper around `Box<dyn PuzzleTypeGpuData>` that can be downcast to a
    /// concrete GPU data type.
    pub struct BoxDynVantageSetEngineData(Box<dyn VantageSetEngineData>);
}
impl VantageSetEngineData for () {}

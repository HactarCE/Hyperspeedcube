mod arena;
mod manifold;
mod shape;
mod simplices;

pub use arena::*;
pub use manifold::*;
pub use shape::*;
pub use simplices::*;

#[cfg(debug_assertions)]
mod log;

#[cfg(not(debug_assertions))]
mod log {
    use std::fmt;
    use tinyset::{Fits64, Set64};

    #[derive(Debug, Default, Clone)]
    pub struct ShapeConstructionLog;
    impl fmt::Display for ShapeConstructionLog {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "shape construction logging is disabled in this build")
        }
    }
    impl ShapeConstructionLog {
        pub fn event(&self, _event_type: &'static str, _msg: impl fmt::Display) -> EventGuard {
            EventGuard
        }
    }

    pub(super) struct EventGuard;
    impl EventGuard {
        pub fn log(&self, _msg: impl fmt::Display) {}
        pub fn log_value(&self, _var_name: &str, _value: impl fmt::Display) {}
        pub fn log_set64<T: fmt::Display>(&self, _var_name: &str, _value: &Set64<T>) {}
        pub fn log_option(&self, _var_name: &str, _value: Option<impl fmt::Display>) {}
    }
}

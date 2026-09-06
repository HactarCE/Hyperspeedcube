//! Collections for geometric constructs.

mod motor_nn;
mod range_map;
mod vecmap;

pub use motor_nn::MotorNearestNeighborMap;
pub use range_map::{NanError, RangeMap};
pub use vecmap::VecMap;

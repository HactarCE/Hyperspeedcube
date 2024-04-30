//! Collections for geometric algebra constructs.

#[macro_use]
pub mod generic_vec;
pub mod approx_hashmap;
mod motor_nn;
mod vecmap;

pub use approx_hashmap::{ApproxHashMap, ApproxHashMapKey};
pub use generic_vec::{GenericVec, IndexNewtype, IndexOutOfRange, IndexOverflow};
pub use motor_nn::MotorNearestNeighborMap;
pub use vecmap::VecMap;

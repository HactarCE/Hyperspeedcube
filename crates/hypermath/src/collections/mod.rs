//! Collections for geometric algebra constructs.

#[macro_use]
pub mod generic_vec;
mod generic_mask;
mod motor_nn;
mod vecmap;

pub use generic_mask::GenericMask;
pub use generic_vec::{GenericVec, IndexNewtype, IndexOutOfRange, IndexOverflow};
pub use motor_nn::MotorNearestNeighborMap;
pub use vecmap::VecMap;

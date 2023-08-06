//! Collections for geometric algebra constructs.

#[macro_use]
pub mod generic_vec;
pub mod approx_hashmap;
mod isometry_nn;
mod vecmap;

pub use approx_hashmap::ApproxHashMap;
pub use generic_vec::GenericVec;
pub use isometry_nn::MultivectorNearestNeighborsMap;
pub use vecmap::VecMap;

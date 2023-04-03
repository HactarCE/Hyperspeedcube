//! Collections for storing with geometric algebra constructs.

#[macro_use]
mod generic_vec;
pub mod approx_hashmap;
mod isometry_hashmap;
mod isometry_nn;
mod vecmap;
mod vector_hashmap;

pub use approx_hashmap::ApproxHashMap;
pub use generic_vec::{GenericVec, IndexNewtype};
pub use isometry_hashmap::IsometryHashMap;
pub use isometry_nn::IsometryNearestNeighborsMap;
pub use vecmap::VecMap;
pub use vector_hashmap::VectorHashMap;

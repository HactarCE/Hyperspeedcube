//! Collections for storing with geometric algebra constructs.

pub mod isometry_hashmap;
mod isometry_nn;

pub use isometry_hashmap::IsometryHashMap;
pub use isometry_nn::IsometryNearestNeighborsMap;

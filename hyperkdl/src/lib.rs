//! KDL serialization library.

/// Re-export of `kdl`.
pub use kdl;
/// Re-export of `miette`.
pub use miette;

mod ctx;
mod schema;
mod schema_impl;

pub use ctx::*;
pub use schema::*;

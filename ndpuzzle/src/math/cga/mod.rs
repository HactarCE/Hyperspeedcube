//! Conformal geometric algebra.
//!
//! https://en.wikipedia.org/wiki/Conformal_geometric_algebra

mod axes;
mod blade;
pub mod collections;
mod isometry;
mod multivector;
mod term;

pub use axes::Axes;
pub use blade::{Blade, Point, PointQueryResult, ToConformalPoint};
pub use isometry::Isometry;
pub use multivector::{AsMultivector, Multivector};
pub use term::Term;

#[cfg(test)]
mod tests;

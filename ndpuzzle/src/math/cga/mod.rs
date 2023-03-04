//! Conformal geometric algebra.
//!
//! https://en.wikipedia.org/wiki/Conformal_geometric_algebra

mod axes;
mod blade;
mod multivector;
mod term;
// mod versor;

pub use axes::Axes;
pub use blade::{Blade, Point, PointQueryResult};
pub use multivector::Multivector;
pub use term::Term;
// pub use versor::{Rotor, Rotoreflector};

#[cfg(test)]
mod tests;

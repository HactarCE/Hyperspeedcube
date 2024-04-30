//! [Conformal geometric algebra](https://w.wiki/7SP3).

mod axes;
mod blade;
mod isometry;
mod multivector;
mod point;
mod tangent;
mod term;

pub use axes::Axes;
pub use blade::{Blade, MismatchedGrade};
pub use isometry::Isometry;
pub use multivector::{AsMultivector, Multivector};
pub use point::{Point, ToConformalPoint};
pub use tangent::TangentSpace;
pub use term::Term;

#[cfg(test)]
mod tests;

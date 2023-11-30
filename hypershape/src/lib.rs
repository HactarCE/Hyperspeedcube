//! Multidimensional shape slicing and other geometric algorithms.
//!
//! In this crate:
//! - A 0-dimensional **manifold** is always a pair of points.
//! - An N-dimensional **manifold** where N>0 is always closed (compact and with
//!   no boundary). More specifically, it is a hyperplane or hypersphere,
//!   represented using an OPNS blade in the [conformal geometric algebra].
//! - The **inside** and **outside** of a manifold are the half-spaces enclosed
//!   by it when embedded with an orientation into another manifold with one
//!   more dimension. In conformal geometry, the inside and outside must be
//!   determined by the orientation of the manifold rather than which half-space
//!   is finite.
//! - An **atomic polytope** in N-dimensional space is the intersection of the
//!   **inside**s of finitely many (N-1)-dimensional manifolds. It is
//!   represented as an N-dimensional manifold (on which the polytope lives) and
//!   a set of oriented (N-1)-dimensional polytopes that bound it.
//!
//! [conformal geometric algebra]: https://w.wiki/7SP3
//!
//! Atomic polytopes are memoized and given IDs.

#![warn(
    clippy::doc_markdown,
    clippy::if_then_some_else_none,
    clippy::manual_let_else,
    clippy::semicolon_if_nothing_returned,
    clippy::semicolon_inside_block,
    clippy::too_many_lines,
    clippy::undocumented_unsafe_blocks,
    clippy::unwrap_used,
    missing_docs,
    rust_2018_idioms
)]

pub mod group;
mod slabmap;
pub mod space;
mod util;

pub use group::*;
use slabmap::SlabMap;
pub use space::*;

/// Structs, traits, and constants.
pub mod prelude {
    pub use crate::group::*;
    pub use crate::space::*;
}

#[cfg(test)]
mod tests;

//! Multidimensional shape slicing and other geometric algorithms.

#![warn(
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
pub mod space;

pub use group::*;
pub use space::*;

/// Structs, traits, and constants.
pub mod prelude {
    pub use crate::group::*;
    pub use crate::space::*;
}

#[cfg(test)]
mod tests;

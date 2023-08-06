//! Multidimensional twisty puzzle generator and simulator backend.

// #![warn(
//     rust_2018_idioms,
//     missing_docs,
//     clippy::cargo,
//     clippy::if_then_some_else_none,
//     clippy::manual_let_else,
//     clippy::semicolon_if_nothing_returned,
//     clippy::semicolon_inside_block,
//     clippy::too_many_lines,
//     clippy::undocumented_unsafe_blocks,
//     clippy::unwrap_used
// )]
#![allow(clippy::multiple_crate_versions)]

mod ext;
mod library;
mod lua;
mod task;

pub use library::PuzzleLibrary;
pub use lua::{drain_logs, load_sandboxed, new_lua};
pub use task::TaskHandle;

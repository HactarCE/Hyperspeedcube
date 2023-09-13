//! Multidimensional twisty puzzle generator and simulator backend.

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

mod library;
mod lua;
mod puzzle;
mod task;

pub use library::{FileData, Library, Object, ObjectData};
pub use lua::{drain_logs, load_sandboxed, new_lua, LuaLogLine};
pub use puzzle::*;
pub use task::TaskHandle;

pub type LayerMaskUint = u32;

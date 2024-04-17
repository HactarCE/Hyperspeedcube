//! Multidimensional twisty puzzle generator and simulator backend.

pub mod builder;
mod library;
pub mod lua;
mod puzzle;
mod task;
mod util;

pub use library::*;
pub use lua::LuaLogLine;
pub use puzzle::*;
pub use task::TaskHandle;

/// Unsigned integer type used for [`LayerMask`].
pub type LayerMaskUint = u32;

/// Version string such as `hyperpuzzle v1.2.3`.
pub const PUZZLE_ENGINE_VERSION_STRING: &str =
    concat!(env!("CARGO_PKG_NAME"), " v", env!("CARGO_PKG_VERSION"));

/// Whether to capture Lua `print()`, `warn()`, and `error()` to prevent them
/// from going to stdout/stderr.
const CAPTURE_LUA_OUTPUT: bool = cfg!(debug_assertions);

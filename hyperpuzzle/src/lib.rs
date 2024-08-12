//! Multidimensional twisty puzzle generator and simulator backend.

#[macro_use]
extern crate lazy_static;

pub mod builder;
mod library;
pub mod lua;
mod puzzle;
mod rgb;
mod task;
pub mod util;

pub use library::*;
pub use lua::LuaLogLine;
pub use puzzle::*;
pub use rgb::Rgb;
pub use task::TaskHandle;

/// Unsigned integer type used for [`LayerMask`].
pub type LayerMaskUint = u32;

/// Version string such as `hyperpuzzle v1.2.3`.
pub const PUZZLE_ENGINE_VERSION_STRING: &str =
    concat!(env!("CARGO_PKG_NAME"), " v", env!("CARGO_PKG_VERSION"));

/// Whether to capture Lua `print()`, `warn()`, and `error()` to prevent them
/// from going to stdout/stderr.
const CAPTURE_LUA_OUTPUT: bool = !cfg!(test);

const MAX_TWIST_REPEAT: usize = 50;

/// Radius of the promordial cube, which determines the maximum extent of all
/// vertices along any axis.
pub const PRIMORDIAL_CUBE_RADIUS: hypermath::Float = 1_048_576.0; // big power of 2 feels good

/// Name of the default color scheme, if no other is specified.
pub const DEFAULT_COLOR_SCHEME_NAME: &str = "Default";
/// Name of the default gradient, to which unknown or conflicting colors are
/// assigned.
pub const DEFAULT_COLOR_GRADIENT_NAME: &str = "Rainbow";

/// Regex matching allowed names for puzzle elements.
///
/// - Digits and `<` are reserved for auto-generated names.
/// - Space and many special symbols are used in piece filter expressions.
/// - Most other symbols are disallowed just for safety and
///   forwards-compatibility.
pub const NAME_REGEX: &str = r"[a-zA-Z_][a-zA-Z0-9_]*";

fn validate_id(s: String) -> eyre::Result<String> {
    if !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || c == '_') {
        Ok(s)
    } else {
        Err(eyre::eyre!(
            "invalid ID; ID must be nonempty and contain \
             only alphanumeric characters and '_'",
        ))
    }
}

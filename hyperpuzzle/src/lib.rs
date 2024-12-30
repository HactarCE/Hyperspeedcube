//! Multidimensional twisty puzzle generator and simulator backend.

#[macro_use]
extern crate lazy_static;

pub mod builder;
mod library;
mod lint;
pub mod lua;
mod puzzle;
mod rgb;
mod tags;
mod timestamp;
pub mod util;

/// Re-export of `chrono`.
pub use chrono;
pub use library::*;
pub use lint::PuzzleLintOutput;
pub use lua::{LuaLogLine, Version};
pub use puzzle::*;
pub use rgb::Rgb;
pub use tags::*;
pub use timestamp::Timestamp;

/// Unsigned integer type used for [`LayerMask`].
pub type LayerMaskUint = u32;

/// Version string such as `hyperpuzzle v1.2.3`.
pub const PUZZLE_ENGINE_VERSION_STRING: &str =
    concat!(env!("CARGO_PKG_NAME"), " v", env!("CARGO_PKG_VERSION"));

/// Whether to capture Lua `print()`, `warn()`, and `error()` to prevent them
/// from going to stdout/stderr.
const CAPTURE_LUA_OUTPUT: bool = !cfg!(test);

const MAX_TWIST_REPEAT: usize = 50;
const MAX_NAME_SET_SIZE: usize = 100;

const MAX_PUZZLE_REDIRECTS: usize = 20;

/// Default length for a full scramble
pub const FULL_SCRAMBLE_LENGTH: u32 = 1000;

/// Radius of the promordial cube, which determines the maximum extent of all
/// vertices along any axis.
pub const PRIMORDIAL_CUBE_RADIUS: hypermath::Float = 1_048_576.0; // big power of 2 feels good

/// Name of the default color scheme, if no other is specified.
pub const DEFAULT_COLOR_SCHEME_NAME: &str = "Default";
/// Name of the default gradient, to which unknown or conflicting colors are
/// assigned.
pub const DEFAULT_COLOR_GRADIENT_NAME: &str = "Rainbow";

/// Returns `s` if it is a valid ID for a shared object (such as a puzzle or
/// color system), or an error if it not.
///
/// Internally, this calls [`validate_id_str()`].
fn validate_id(s: String) -> eyre::Result<String> {
    validate_id_str(&s).map(|_| s)
}

/// Returns an error if `s` is not a valid ID for a shared object (such as a
/// puzzle or color system).
fn validate_id_str(s: &str) -> eyre::Result<()> {
    if !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || c == '_') {
        Ok(())
    } else {
        Err(eyre::eyre!(
            "invalid ID {s:?}; ID must be nonempty and \
             contain only alphanumeric characters and '_'",
        ))
    }
}

/// Parses the ID of a generated puzzle into its components: the generator ID,
/// and the parameters. Returns `None` if the ID is not a valid ID for a
/// generated puzzle.
pub fn parse_generated_puzzle_id(id: &str) -> Option<(&str, Vec<&str>)> {
    let (generator_id, args) = id.split_once(':')?;
    Some((generator_id, args.split(',').collect()))
}

/// Returns the ID of a generated puzzle.
pub fn generated_puzzle_id(
    generator_id: &str,
    params: impl IntoIterator<Item = impl ToString>,
) -> String {
    let mut ret = generator_id.to_owned();
    let mut is_first = true;
    for param in params {
        ret += if is_first { ":" } else { "," };
        is_first = false;
        ret += &param.to_string();
    }
    ret
}

fn compare_puzzle_ids(a: &str, b: &str) -> std::cmp::Ordering {
    if a == b {
        return std::cmp::Ordering::Equal;
    }

    let Some((a_id, a_params)) = crate::parse_generated_puzzle_id(a) else {
        return a.cmp(b);
    };
    let Some((b_id, b_params)) = crate::parse_generated_puzzle_id(b) else {
        return a.cmp(b);
    };

    match a_id.cmp(b_id) {
        std::cmp::Ordering::Equal => (),
        ord => return ord,
    }

    fn parse_float_else_str(s: &str) -> Result<float_ord::FloatOrd<f64>, &str> {
        match s.parse::<f64>() {
            Ok(n) => Ok(float_ord::FloatOrd(n)),
            Err(_) => Err(s),
        }
    }

    Iterator::cmp(
        a_params.into_iter().map(parse_float_else_str),
        b_params.into_iter().map(parse_float_else_str),
    )
}

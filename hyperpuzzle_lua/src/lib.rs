//! Multidimensional twisty puzzle generator and simulator backend.
//!
//! TODO: document `hyperpaths` optional dependency

use std::path::Path;

#[macro_use]
extern crate lazy_static;

pub mod builder;
pub mod lua;

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

#[cfg(feature = "hyperpaths")]
const BAKE_LUA_PATHS: bool = hyperpaths::IS_OFFICIAL_BUILD;
#[cfg(not(feature = "hyperpaths"))]
const BAKE_LUA_PATHS: bool = true;

/// Built-in Lua files.
static LUA_BUILTIN_DIR: include_dir::Dir<'_> = if BAKE_LUA_PATHS {
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/../lua")
} else {
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/resources/lua")
};

/// Loads all puzzles defined using Lua.
pub fn load_puzzles(catalog: &hyperpuzzle_core::Catalog, logger: &hyperpuzzle_core::Logger) {
    let Ok(loader) = lua::LuaLoader::new(catalog, logger) else {
        log::error!("error initializing Lua loader");
        log::error!("no Lua files will be loaded");
        return;
    };

    // Load built-in puzzles.
    log::info!("reading built-in Lua files");
    loader.read_builtin_directory();

    // Load user puzzles.
    #[cfg(feature = "hyperpaths")]
    match hyperpaths::lua_dir() {
        Ok(lua_dir) => {
            log::info!("reading Lua files from path {}", lua_dir.to_string_lossy());
            loader.read_directory(lua_dir);
        }
        Err(e) => {
            log::error!("error locating Lua directory: {e}");
        }
    }

    loader.load_all_files(logger);
}

pub fn extract_builtin_files(base_path: &Path) -> std::io::Result<()> {
    LUA_BUILTIN_DIR.extract(base_path)
}

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

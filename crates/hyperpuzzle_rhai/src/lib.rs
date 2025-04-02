//! Rhai API for defining puzzles for Hyperspeedcube.

use std::path::Path;

mod loader;
mod package;

#[cfg(feature = "hyperpaths")]
const BAKE_RHAI_PATHS: bool = hyperpaths::IS_OFFICIAL_BUILD;
#[cfg(not(feature = "hyperpaths"))]
const BAKE_RHAI_PATHS: bool = true;

/// Built-in Rhai files.
static RHAI_BUILTIN_DIR: include_dir::Dir<'_> = if BAKE_RHAI_PATHS {
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/../../rhai")
} else {
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/resources/rhai")
};

const MAX_TWIST_REPEAT: usize = 50;

/// Loads all puzzles defined using Rhai.
pub fn load_puzzles(catalog: &hyperpuzzle_core::Catalog, logger: &hyperpuzzle_core::Logger) {
    loader::load_files_with_new_engine(catalog, logger);
}

/// Extracts the built-in Rhai files to the specified path.
pub fn extract_builtin_files(base_path: &Path) -> std::io::Result<()> {
    RHAI_BUILTIN_DIR.extract(base_path)
}

#[test]
fn test_load() {
    load_puzzles(
        &hyperpuzzle_core::Catalog::new(),
        &hyperpuzzle_core::Logger::new(),
    );
}

//! Domain-specific language for defining puzzles for Hyperspeedcube.

#![warn(variant_size_differences)]

mod ast;
mod builtins;
mod diagnostic;
mod parse;
mod runtime;
mod ty;
mod util;
mod value;

pub use runtime::{EvalCtx, FileStore, Runtime, Scope};
use std::path::Path;

pub use diagnostic::{DiagMsg, FullDiagnostic, ImmutReason, TracebackLine};
pub use ty::{FnType, Type};
use value::{FnDebugInfo, FnOverload, FnValue, Index, MapKey, Value, ValueData};

pub type Result<T, E = FullDiagnostic> = std::result::Result<T, E>;

/// Numeric ID for a Hyperpuzzlescript file.
pub type FileId = u32;
/// Span in a Hyperpuzzlescript file.
pub type Span = chumsky::span::SimpleSpan<u32, FileId>;
/// Value with an associated `Span`.
pub type Spanned<T> = (T, Span);

const BUILTIN_SPAN: Span = Span {
    start: 0,
    end: 0,
    context: FileId::MAX,
};

#[cfg(feature = "hyperpaths")]
const BAKE_HPS_PATHS: bool = hyperpaths::IS_OFFICIAL_BUILD;
#[cfg(not(feature = "hyperpaths"))]
const BAKE_HPS_PATHS: bool = true;

/// Built-in Hyperpuzzlescript files.
static HPS_BUILTIN_DIR: include_dir::Dir<'_> = if BAKE_HPS_PATHS {
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/../../hps")
} else {
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/resources/hps")
};

/// Loads all puzzles defined using Hyperpuzzlescript.
pub fn load_puzzles(catalog: &hyperpuzzle_core::Catalog, logger: &hyperpuzzle_core::Logger) {
    Runtime::with_default_files().exec_all_files();
}

/// Extracts the built-in Hyperpuzzlescript files to the specified path.
pub fn extract_builtin_files(base_path: &Path) -> std::io::Result<()> {
    HPS_BUILTIN_DIR.extract(base_path)
}

/// Maximum period of a twist.
const MAX_TWIST_REPEAT: usize = 50;

#[cfg(test)]
mod tests;

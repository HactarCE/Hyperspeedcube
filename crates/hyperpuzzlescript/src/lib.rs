//! Domain-specific language for defining puzzles for Hyperspeedcube.

#![warn(variant_size_differences)]

mod ast;
// mod builtins;
mod diagnostic;
// mod eval;
// mod file_store;
mod parse;
mod runtime;
mod ty;
mod util;
// mod value;

pub use runtime::{FileStore, Runtime};
use std::path::Path;

pub use diagnostic::{Error, ErrorMsg, Warning};
// use eval::Ctx;
pub use ty::{FnType, Type};
// use value::Value;

/// Numeric ID for a Hyperpuzzlescript file.
pub type FileId = u32;
/// Span in a Hyperpuzzlescript file.
pub type Span = chumsky::span::SimpleSpan<u32, FileId>;
/// Value with an associated `Span`.
pub type Spanned<T> = (T, Span);

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
    // runtime::load_files_with_new_engine(catalog, logger);
    todo!()
}

/// Extracts the built-in Hyperpuzzlescript files to the specified path.
pub fn extract_builtin_files(base_path: &Path) -> std::io::Result<()> {
    HPS_BUILTIN_DIR.extract(base_path)
}

/// Maximum period of a twist.
const MAX_TWIST_REPEAT: usize = 50;

#[test]
pub fn test_eval() {
    let src = include_str!("../resources/hps/polygonal.hps");
    // let ast = parser::parse(src).unwrap();
    // let mut ctx = Ctx {
    //     src: src.into(),
    //     globals: HashMap::new(),
    // };
    // println!("{:?}", ctx.eval(&ast))
}

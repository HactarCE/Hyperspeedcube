//! Domain-specific language for defining puzzles for Hyperspeedcube.

#![warn(variant_size_differences)]

#[macro_use]
pub mod util;
mod ast;
pub mod builtins;
pub mod codegen;
mod custom_value;
mod diagnostic;
mod engines;
mod parse;
mod request;
mod runtime;
mod ty;
mod value;

pub use ast::SpecialVar;
pub use custom_value::{BoxDynValue, CustomValue, TryEq};
use diagnostic::LoopControlFlow;
pub use diagnostic::{
    Diagnostic, Error, ErrorExt, FullDiagnostic, ImmutReason, TracebackLine, Warning,
};
pub use engines::EngineCallback;
pub use request::EvalRequestTx;
pub use runtime::{EvalCtx, Modules, ParentScope, Runtime, Scope, SpecialVariables};
pub use ty::{FnType, Type};
pub use util::{FromValue, FromValueRef, TypeOf, hps_ty};
pub use value::{FnDebugInfo, FnOverload, FnValue, Value, ValueData};

/// Result type supporting a single [`FullDiagnostic`].
pub type Result<T, E = FullDiagnostic> = std::result::Result<T, E>;

/// Type used for [`ValueData::Num`].
pub type Num = f64;
/// Type used for [`ValueData::Str`].
pub type Str = ecow::EcoString;
/// Type used for [`ValueData::List`].
pub type List = Vec<Value>;
/// Type used for [`ValueData::List`] with a specific type.
pub type ListOf<T> = Vec<Spanned<T>>;
/// Type used for [`ValueData::Map`].
pub type Map = indexmap::IndexMap<Key, Value>;

/// Type used for keys in [`ValueData::Map`].
pub type Key = arcstr::Substr;

/// Numeric ID for a Hyperpuzzlescript file.
pub type FileId = u32;
/// Span in a Hyperpuzzlescript file.
pub type Span = chumsky::span::SimpleSpan<u32, FileId>;
/// Value with an associated `Span`.
pub type Spanned<T> = (T, Span);

/// Dummy span used for built-in values.
///
/// This is handled specially by any code that prints spans.
pub const BUILTIN_SPAN: Span = Span {
    start: 0,
    end: 0,
    context: FileId::MAX,
};

/// Whether to check for function overload conflicts in built-ins and user code.
///
/// This only affects checks performed when the function is defined, not when
/// the function is called.
const CHECK_FN_OVERLOAD_CONFLICTS: bool = true;

/// Name of the scripting language.
pub const LANGUAGE_NAME: &str = "Hyperpuzzlescript";
/// File extension for scripts in the language.
pub const FILE_EXTENSION: &str = "hps";
/// Name of the "index" script in a directory, not including the file extension.
pub const INDEX_FILE_NAME: &str = "index";

/// Name of the `nd_euclid` engine.
pub const ND_EUCLID: &str = "euclid";

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

/// Extracts the built-in Hyperpuzzlescript files to the specified path.
pub fn extract_builtin_files(base_path: &std::path::Path) -> std::io::Result<()> {
    HPS_BUILTIN_DIR.extract(base_path)
}

/// Maximum period of a twist.
const MAX_TWIST_REPEAT: usize = 50;

#[cfg(test)]
mod tests;

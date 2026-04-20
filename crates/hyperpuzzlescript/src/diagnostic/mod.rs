mod error;
mod full;
mod report;
mod traceback;
mod warning;

pub use error::{AstSyntaxError, Error, ErrorExt, ImmutReason};
pub(crate) use full::LoopControlFlow;
pub use full::{FormattedFullDiagnostic, FullDiagnostic};
use report::ReportBuilder;
pub use traceback::TracebackLine;
pub use warning::Warning;

/// [`Error`] or [`Warning`], without traceback information.
#[derive(thiserror::Error, Debug, Clone)]
#[expect(missing_docs)]
pub enum Diagnostic {
    #[error("error: {0}")]
    Error(#[from] Error),
    #[error("warning: {0}")]
    Warning(#[from] Warning),
}

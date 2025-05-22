mod error;
mod full;
mod report;
mod traceback;
mod warning;

pub use error::{Error, ImmutReason};
pub use full::FullDiagnostic;
pub(crate) use full::LoopControlFlow;
use report::ReportBuilder;
pub use traceback::TracebackLine;
pub use warning::Warning;

/// [`Error`] or [`Warning`], without traceback information.
#[derive(thiserror::Error, Debug, Clone)]
#[allow(missing_docs)]
pub enum Diagnostic {
    #[error("error: {0}")]
    Error(#[from] Error),
    #[error("warning: {0}")]
    Warning(#[from] Warning),
}

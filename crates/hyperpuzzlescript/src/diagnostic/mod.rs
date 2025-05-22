mod error;
mod report;

pub use error::{DiagMsg, FullDiagnostic, ImmutReason, TracebackLine};
use report::ReportBuilder;

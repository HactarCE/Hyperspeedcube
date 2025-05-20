mod error;
mod report;
mod warning;

pub use error::{Error, ErrorMsg, ImmutReason};
use report::ReportBuilder;
pub use warning::Warning;

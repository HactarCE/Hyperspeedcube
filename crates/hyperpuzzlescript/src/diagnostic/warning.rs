use ecow::EcoString;

use super::{FullDiagnostic, ReportBuilder};
use crate::Span;

/// Warning message, without traceback information.
#[derive(thiserror::Error, Debug, Clone)]
#[non_exhaustive]
#[allow(missing_docs)]
pub enum Warning {
    #[error("{0}")]
    User(EcoString),
}

impl Warning {
    /// Adds a primary span to the warning.
    pub fn at(self, span: impl Into<Span>) -> FullDiagnostic {
        FullDiagnostic {
            msg: self.into(),
            span: span.into(),
            traceback: vec![],
        }
    }

    pub(super) fn report(&self, report_builder: ReportBuilder) -> ReportBuilder {
        match self {
            Self::User(_) => report_builder.main_label("warning reported here"),
        }
    }
}

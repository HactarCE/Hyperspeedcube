use ecow::EcoString;

use super::{FullDiagnostic, ReportBuilder};
use crate::{Key, Span, Spanned};

/// Warning message, without traceback information.
#[derive(thiserror::Error, Debug, Clone)]
#[non_exhaustive]
#[allow(missing_docs)]
pub enum Warning {
    #[error("{0}")]
    User(EcoString),

    #[error("variable is shadowed")]
    ShadowedVariable(Spanned<Key>, bool),
    #[error("export is shadowed")]
    ShadowedExport(Spanned<Key>),
}

impl From<EcoString> for Warning {
    fn from(value: EcoString) -> Self {
        Warning::User(value)
    }
}
impl From<&str> for Warning {
    fn from(value: &str) -> Self {
        Warning::User(value.into())
    }
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
            Self::ShadowedVariable((shadowed_name, original_span), blame_milo) => report_builder
                .main_label(format!("this shadows \x02{shadowed_name}\x03"))
                .label(original_span, "originally defined here")
                .note("while the new variable is in scope, the original one will be inaccessible")
                .notes(blame_milo.then_some(
                    "this may be intentional, but Milo Jacquet asked for \
                     this warning and currently there's no way to supress it",
                ))
                .help("try renaming one of them"),
            Self::ShadowedExport((shadowed_name, original_span)) => report_builder
                .main_label(format!("this shadows \x02{shadowed_name}\x03"))
                .label(original_span, "originally defined here")
                .help("store the result in a variable, then export it once"),
        }
    }
}

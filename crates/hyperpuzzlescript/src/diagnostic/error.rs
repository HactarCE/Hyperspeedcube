use arcstr::Substr;
use thiserror::Error;

use crate::{
    FileId, FnType, Span, Type,
    parse::{LexError, ParseError},
};

use super::ReportBuilder;

/// Error type for the language.
#[derive(Debug, Clone)]
pub struct Error {
    /// Primary span.
    ///
    /// `msg` may contain more spans.
    pub span: Span,
    /// Error message.
    pub msg: ErrorMsg,
}
impl Error {
    /// Returns the error as a string with ANSI escape codes.
    pub fn to_string(&self, files: impl ariadne::Cache<FileId>) -> String {
        self.report().to_string_with_ansi_escapes(files)
    }

    fn report(&self) -> ReportBuilder {
        let (code, msg) = self.msg.code_and_msg_str();
        let report_builder = ReportBuilder::error(code, msg, self.span);

        match &self.msg {
            ErrorMsg::LexError(e) => report_builder.main_label(e.reason()).labels(
                e.contexts()
                    .map(|(pat, sp)| ((self.span.context, *sp), pat)),
            ),
            ErrorMsg::ParseError(e) => report_builder
                .main_label(e.reason())
                .labels(e.contexts().map(|(pat, sp)| (sp, pat))),
            ErrorMsg::Unimplemented(_) => {
                report_builder.main_label("this feature isn't implemented yet")
            }
            ErrorMsg::TypeError { expected, got } => {
                report_builder.main_label(format!("expected {expected}, got {got}"))
            }
            ErrorMsg::Immut { reason } => {
                report_builder.main_label(format!("this cannot be modified because {reason}"))
            }
            ErrorMsg::FnOverloadConflict {
                new_ty,
                old_ty,
                old_span,
            } => report_builder
                .main_label(format!("this has type {new_ty}"))
                .label(old_span, format!("previous overload has type {old_ty}")),
        }
    }
}

/// Error type for the language, without a primary span.
#[derive(Debug, Clone)]
#[non_exhaustive]
#[allow(missing_docs)]
pub enum ErrorMsg {
    LexError(LexError<'static>),
    ParseError(ParseError<'static>),
    Unimplemented(&'static str),
    TypeError {
        expected: Type,
        got: Type,
    },
    Immut {
        reason: ImmutReason,
    },
    FnOverloadConflict {
        new_ty: Box<FnType>,
        old_ty: Box<FnType>,
        old_span: Span,
    },
}
impl ErrorMsg {
    /// Adds a primary span to the error.
    pub fn at(self, span: impl Into<Span>) -> Error {
        Error {
            span: span.into(),
            msg: self,
        }
    }

    fn code_and_msg_str(&self) -> (u32, &'static str) {
        match self {
            Self::LexError(_) | Self::ParseError(_) => (1, "syntax error"),
            Self::Unimplemented(_) => (2, "unimplemented"),
            Self::TypeError { .. } => (3, "type error"),
            Self::Immut { .. } => (4, "immutable value"),
            Self::FnOverloadConflict { .. } => (5, "conflicting function overload"),
        }
    }
}

/// Reason for a variable being immutable.
#[derive(Error, Debug, Clone)]
pub enum ImmutReason {
    #[error("it is built into the language")]
    Builtin,
    #[error("it is defined outside the function `{0}`")]
    NamedFn(Substr),
    #[error("it is defined outside the current function")]
    AnonymousFn,
}

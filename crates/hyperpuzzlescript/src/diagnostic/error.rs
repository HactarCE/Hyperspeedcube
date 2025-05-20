use arcstr::Substr;
use itertools::Itertools;
use thiserror::Error;

use crate::{
    FileId, FnDebugInfo, FnType, Span, Type, Value,
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
            ErrorMsg::ExpectedType { ast_node_kind } => report_builder
                .main_label(format!("this is a {ast_node_kind}"))
                .help("try a type like `Str` or `List[Num]`"),
            ErrorMsg::ExpectedCollectionType { .. } => report_builder
                .main_label(format!("this is not a collection type"))
                .help("try a collection type like `List` or `Map`"),
            ErrorMsg::FnOverloadConflict {
                new_ty,
                old_ty,
                old_span,
            } => report_builder
                .main_label(format!("this has type {new_ty}"))
                .label_or_note(*old_span, format!("previous overload has type {old_ty}")),
            ErrorMsg::CannotAssignToExpr { kind } => {
                report_builder.main_label(format!("{kind} is not assignable"))
            }
            ErrorMsg::Undefined => report_builder
                .main_label("this is undefined")
                .help("it may be defined somewhere, but isn't accessible from here")
                .help("try assigning `null` to the variable earlier"),
            ErrorMsg::UnknownType => report_builder
                .main_label("unknown type")
                .help("try using a type name like `Num` or `Str`"),
            ErrorMsg::WrongNumberOfIndices { count, min, max } => report_builder
                .main_label(format!("found {count} index expression(s)"))
                .help(if min == max {
                    format!("there should be exactly {min}")
                } else {
                    format!("there should be between {min} and {max} (inclusive)")
                }),
            ErrorMsg::UnsupportedOperator => report_builder
                .main_label("this operator is not supported")
                .help("contact the developer if you have a use case for it"),
            ErrorMsg::NoFnWithName => report_builder.main_label("no function with this name"),
            ErrorMsg::BadArgTypes(candidates) => report_builder
                .main_label("bad argument types")
                .help(if candidates.is_empty() {
                    "this function cannot be called".to_owned()
                } else {
                    format!("try one of these:\n{}", candidates.iter().join("\n"))
                }),
            ErrorMsg::AmbiguousFnCall(candidates) => {
                report_builder.main_label("ambiguous function call").notes(
                    candidates
                        .iter()
                        .map(|candidate| format!("could be {candidate}")),
                )
            }
            ErrorMsg::ExpectedMapKey => report_builder
                .main_label("expected map key")
                .help("convert a value to a string to use it as a map key: `\"${my_value}\"`"),
            ErrorMsg::BreakOutsideLoop | ErrorMsg::ContinueOutsideLoop => {
                report_builder.main_label("not in a loop")
            }

            ErrorMsg::AstErrorNode => report_builder.help("see earlier errors for a syntax error"),
            ErrorMsg::Internal(msg) => report_builder.main_label(msg),

            ErrorMsg::Return(_) | ErrorMsg::Break | ErrorMsg::Continue => {
                report_builder.main_label("leaked control flow")
            }
        }
    }

    pub fn try_resolve_return_value(mut self) -> Result<Value, Self> {
        match &mut self.msg {
            ErrorMsg::Return(ret_val) => Ok(std::mem::take(ret_val)),
            ErrorMsg::Break => Err(ErrorMsg::BreakOutsideLoop.at(self.span)),
            ErrorMsg::Continue => Err(ErrorMsg::ContinueOutsideLoop.at(self.span)),
            _ => Err(self),
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
    ExpectedType {
        ast_node_kind: &'static str,
    },
    ExpectedCollectionType,
    Immut {
        reason: ImmutReason,
    },
    FnOverloadConflict {
        new_ty: Box<FnType>,
        old_ty: Box<FnType>,
        old_span: Option<Span>,
    },
    CannotAssignToExpr {
        kind: &'static str,
    },
    Undefined,
    UnknownType,
    WrongNumberOfIndices {
        count: usize,
        min: usize,
        max: usize,
    },
    UnsupportedOperator,
    NoFnWithName,
    BadArgTypes(Vec<FnType>),
    AmbiguousFnCall(Vec<FnType>),
    ExpectedMapKey,
    BreakOutsideLoop,
    ContinueOutsideLoop,

    // TODO: how to handle this
    AstErrorNode,
    Internal(&'static str),

    /// Not actually an error; used for ordinary return statements.
    Return(Box<Value>),
    /// Not actually an error; used for ordinary break statements.
    Break,
    /// Not actually an error; used for ordinary continue statements.
    Continue,
}
impl ErrorMsg {
    /// Adds a primary span to the error.
    pub fn at(self, span: impl Into<Span>) -> Error {
        Error {
            span: span.into(),
            msg: self,
        }
    }

    pub fn debug_at(self, debug: FnDebugInfo) -> Error {
        match debug {
            FnDebugInfo::Span(span) => self.at(span),
            FnDebugInfo::Internal(name) => {
                if cfg!(debug_assertions) {
                    panic!("error in internal {name:?}: {self:?}")
                } else {
                    self.at(crate::BUILTIN_SPAN)
                }
            }
        }
    }

    fn code_and_msg_str(&self) -> (u32, &'static str) {
        match self {
            Self::LexError(_) | Self::ParseError(_) => (1, "syntax error"),
            Self::Unimplemented(_) => (2, "unimplemented"),
            Self::TypeError { .. } => (3, "type error"),
            Self::Immut { .. } => (4, "immutable value"),
            Self::ExpectedType { .. } => (5, "expected type"),
            Self::ExpectedCollectionType { .. } => (6, "expected collection type"),
            Self::FnOverloadConflict { .. } => (7, "conflicting function overload"),
            Self::CannotAssignToExpr { .. } => (8, "non-assignable expression"),
            Self::Undefined => (9, "variable is undefined"),
            Self::UnknownType => (10, "unknown type"),
            Self::WrongNumberOfIndices { .. } => (11, "wrong number of indices"),
            Self::UnsupportedOperator => (12, "unsupported operator"),
            Self::NoFnWithName => (13, "no function with name"),
            Self::BadArgTypes(_) => (14, "bad argument types"),
            Self::AmbiguousFnCall(_) => (15, "ambiguous function call"),
            Self::ExpectedMapKey => (16, "expected map key"),
            Self::BreakOutsideLoop => (17, "'break' used outside loop"),
            Self::ContinueOutsideLoop => (18, "'continue' used outside loop"),

            Self::AstErrorNode => (99, "syntax error prevents evaluation"),
            Self::Internal(_) => (100, "internal error"),

            Self::Return(_) | Self::Break | Self::Continue => (100, "internal error"),
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

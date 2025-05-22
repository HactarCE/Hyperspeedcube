use arcstr::Substr;
use ariadne::Fmt;
use ecow::EcoString;
use itertools::Itertools;
use thiserror::Error;

use crate::{
    FileId, FnDebugInfo, FnType, Span, Spanned, Type, Value,
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
    /// Caller spans.
    pub traceback: Vec<TracebackLine>,
    /// Error message.
    pub msg: ErrorMsg,
}
impl Error {
    /// Adds a caller span to the error message.
    pub fn at_caller(mut self, traceback_line: TracebackLine) -> Error {
        self.traceback.push(traceback_line);
        self
    }

    /// Returns the error as a string with ANSI escape codes.
    pub fn to_string(&self, mut files: impl ariadne::Cache<FileId>) -> String {
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
                report_builder.main_label(format!("expected \x02{expected}\x03, got \x02{got}\x03"))
            }
            ErrorMsg::Immut { reason } => {
                report_builder.main_label(format!("this cannot be modified because {reason}"))
            }
            ErrorMsg::ExpectedType { ast_node_kind } => report_builder
                .main_label(format!("this is a \x02{ast_node_kind}\x03"))
                .help("try a type like `Str` or `List[Num]`"),
            ErrorMsg::ExpectedCollectionType { .. } => report_builder
                .main_label(format!("this is not a collection type"))
                .help("try a collection type like `List` or `Map`"),
            ErrorMsg::FnOverloadConflict {
                new_ty,
                old_ty,
                old_span,
            } => report_builder
                .main_label(format!("this has type \x02{new_ty}\x03"))
                .label_or_note(
                    *old_span,
                    format!("previous overload has type \x02{old_ty}\x03"),
                )
                .note("overloads may be ambiguous when passed an empty `List` or `Map`"),
            ErrorMsg::CannotAssignToExpr { kind } => {
                report_builder.main_label(format!("\x02{kind}\x03 is not assignable"))
            }
            ErrorMsg::Undefined => report_builder
                .main_label("this is undefined")
                .help("it may be defined somewhere, but isn't accessible from here")
                .help("try assigning `null` to define the variable in an outer scope"),
            ErrorMsg::UnknownType => report_builder
                .main_label("unknown type")
                .help("try using a type name like `Num` or `Str`"),
            ErrorMsg::WrongNumberOfIndices {
                obj_span,
                count,
                min,
                max,
            } => report_builder
                .main_label(format!("found {count} index value(s)"))
                .label(obj_span, "when indexing this")
                .help(there_should_be_min_max_msg(*min, *max)),
            ErrorMsg::WrongNumberOfLoopVars {
                iter_span,
                count,
                min,
                max,
            } => report_builder
                .main_label(format!("found {count} loop variable(s)"))
                .label(iter_span, "when iterating over this")
                .help(there_should_be_min_max_msg(*min, *max)),
            ErrorMsg::UnsupportedOperator => report_builder
                .main_label("this operator is not supported")
                .help("contact the developer if you have a use case for it"),
            ErrorMsg::NoFnWithName => report_builder.main_label("no function with this name"),
            ErrorMsg::BadArgTypes {
                arg_types,
                overloads,
            } => report_builder
                .main_label("for this function")
                .label_types(arg_types)
                .help(if overloads.is_empty() {
                    "this function cannot be called".to_owned()
                } else {
                    format!("try one of these:\n{}", overloads.iter().join("\n"))
                }),
            ErrorMsg::AmbiguousFnCall {
                arg_types,
                overloads,
            } => report_builder
                .main_label("for this function")
                .label_types(arg_types)
                .notes(
                    overloads
                        .iter()
                        .map(|candidate| format!("could be {candidate}")),
                ),
            ErrorMsg::ExpectedMapKey => report_builder
                .main_label("expected map key")
                .help("convert a value to a string to use it as a map key: `\"${my_value}\"`"),
            ErrorMsg::BreakOutsideLoop | ErrorMsg::ContinueOutsideLoop => {
                report_builder.main_label("not in a loop")
            }
            ErrorMsg::NoField { obj } => report_builder
                .main_label("this field does not exist")
                .label(obj, "on this object"),
            ErrorMsg::CannotSetField { obj } => report_builder
                .main_label("cannot set this field")
                .label(obj, "on this object"),
            ErrorMsg::NormalizeZeroVector => report_builder.main_label("vector is zero"),
            ErrorMsg::InvalidComparison(ty1, ty2) => report_builder
                .main_label("this comparison operator is unsupported on those types")
                .label_type(ty1)
                .label_type(ty2),
            ErrorMsg::ExpectedInteger(n) => report_builder.main_label(n),
            ErrorMsg::IndexOutOfBounds { got, bounds } => {
                report_builder.main_label(got).note(match bounds {
                    Some((min, max)) => {
                        format!("expected integer between {min} and {max} (inclusive)")
                    }
                    None => format!("collection is empty"),
                })
            }
            ErrorMsg::CannotIndex(ty) => {
                report_builder.main_label(format!("this is a \x02{ty}\x03"))
            }
            ErrorMsg::CannotIterate(ty) => {
                report_builder.main_label(format!("this is a \x02{ty}\x03"))
            }

            ErrorMsg::AstErrorNode => report_builder.help("see earlier errors for a syntax error"),
            ErrorMsg::Internal(msg) => report_builder.main_label(msg),
            ErrorMsg::User(_) => report_builder.main_label("error reported here"),
            ErrorMsg::Assert(_) => report_builder.main_label("error reported here"),
            ErrorMsg::AssertCompare(l, r, _) => report_builder.label_values([&**l, &**r]),

            ErrorMsg::Return(_) | ErrorMsg::Break | ErrorMsg::Continue => {
                report_builder.main_label("leaked control flow")
            }
        }
        .labels(
            // only report first caller span
            self.traceback
                .first()
                .filter(|line| line.call_span != self.span)
                .map(|line| (line.call_span, "in this function call")),
        )
        .notes((!self.traceback.is_empty()).then(|| {
            let mut s = "here is the traceback:"
                .fg(ariadne::Color::Fixed(231))
                .to_string();
            for (i, line) in self.traceback.iter().enumerate() {
                line.write(&mut files, &mut s, i == 0, i + 1 == self.traceback.len());
            }
            s
        }))
        .to_string_with_ansi_escapes(files)
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

#[derive(Debug, Clone)]
pub struct TracebackLine {
    pub fn_name: Option<Substr>,
    pub fn_span: Option<Span>,
    pub call_span: Span,
}
impl TracebackLine {
    pub fn write(
        &self,
        mut files: impl ariadne::Cache<FileId>,
        out: &mut String,
        is_first: bool,
        is_last: bool,
    ) {
        *out += if is_first { "\n┬\n" } else { "\n│\n" };
        *out += if is_last { "╰─ " } else { "├─ " };
        match &self.fn_name {
            Some(name) => *out += &name.fg(ariadne::Color::Fixed(49)).to_string(),
            None => *out += "<anonymous fn>",
        }
        if let Some(span) = self.fn_span {
            *out += " (function defined at ";
            *out += &display_span(span, &mut files)
                .fg(ariadne::Color::Fixed(229))
                .to_string();
            *out += ")";
        } else {
            *out += " (built-in function)"
        }
        *out += if is_last { "\n   " } else { "\n│  " };
        *out += "  called at ";
        *out += &display_span(self.call_span, &mut files)
            .fg(ariadne::Color::Fixed(140))
            .to_string();
    }
}

fn display_span(span: Span, mut files: impl ariadne::Cache<FileId>) -> String {
    // Display file name
    match files.display(&span.context) {
        Some(name) => {
            // IIFE to mimic try_block
            let location_suffix = (|| {
                let source = files.fetch(&span.context).ok()?;
                let (line, line_idx, col_idx) = source.get_byte_line(span.start as usize)?;
                let line_text = source.get_line_text(line).unwrap();
                let col_char_idx = line_text[..col_idx.min(line_text.len())].chars().count();
                let line_number = line_idx + 1 + source.display_line_offset();
                let column_number = col_char_idx + 1;
                Some(format!(":{line_number}:{column_number}"))
            })()
            .unwrap_or(String::new());

            format!("{name}{location_suffix}")
        }
        None => "<internal>".to_string(),
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
        obj_span: Span,
        count: usize,
        min: usize,
        max: usize,
    },
    WrongNumberOfLoopVars {
        iter_span: Span,
        count: usize,
        min: usize,
        max: usize,
    },
    UnsupportedOperator,
    NoFnWithName,
    BadArgTypes {
        arg_types: Vec<Spanned<Type>>,
        overloads: Vec<FnType>,
    },
    AmbiguousFnCall {
        arg_types: Vec<Spanned<Type>>,
        overloads: Vec<FnType>,
    },
    ExpectedMapKey,
    BreakOutsideLoop,
    ContinueOutsideLoop,
    NoField {
        obj: Span,
    },
    CannotSetField {
        obj: Span,
    },
    NormalizeZeroVector,
    InvalidComparison(Box<Spanned<Type>>, Box<Spanned<Type>>),
    ExpectedInteger(f64),
    IndexOutOfBounds {
        got: i64,
        bounds: Option<(i64, i64)>,
    },
    CannotIndex(Type),
    CannotIterate(Type),

    // TODO: make sure we're handling AST error node properly
    AstErrorNode,
    Internal(&'static str),
    User(EcoString),
    Assert(EcoString),
    AssertCompare(Box<Value>, Box<Value>, EcoString),

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
            traceback: vec![],
            msg: self,
        }
    }

    /// Reports an error, using `FnDebugInfo` instead of a `Span`.
    ///
    /// This produces slightly error messages on built-in functions, and panics
    /// in debug mode.
    pub fn debug_at(self, debug: FnDebugInfo) -> Error {
        match debug {
            FnDebugInfo::Span(span) => self.at(span),
            FnDebugInfo::Internal(name) => {
                if cfg!(debug_assertions) && false {
                    panic!("error in internal {name:?}: {self:?}")
                } else {
                    self.at(crate::BUILTIN_SPAN)
                }
            }
        }
    }

    pub fn code_and_msg_str(&self) -> (u32, &str) {
        match self {
            Self::LexError(_) | Self::ParseError(_) => (1, "syntax error"),
            Self::Unimplemented(_) => (2, "unimplemented"),
            Self::TypeError { .. } => (3, "type error"),
            Self::Immut { .. } => (4, "immutable value"),
            Self::ExpectedType { .. } => (5, "expected type"),
            Self::ExpectedCollectionType { .. } => (6, "expected collection type"),
            Self::FnOverloadConflict { .. } => (7, "conflicting function overload"),
            Self::CannotAssignToExpr { .. } => (8, "non-assignable expression"),
            Self::Undefined => (9, "undefined"),
            Self::UnknownType => (10, "unknown type"),
            Self::WrongNumberOfIndices { .. } => (11, "wrong number of indices"),
            Self::WrongNumberOfLoopVars { .. } => (12, "wrnog number of loop variables"),
            Self::UnsupportedOperator => (13, "unsupported operator"),
            Self::NoFnWithName => (14, "no function with name"),
            Self::BadArgTypes { .. } => (15, "bad argument types"),
            Self::AmbiguousFnCall { .. } => (16, "ambiguous function call"),
            Self::ExpectedMapKey => (17, "expected map key"),
            Self::BreakOutsideLoop => (18, "'break' used outside loop"),
            Self::ContinueOutsideLoop => (19, "'continue' used outside loop"),
            Self::NoField { .. } => (20, "field does not exist"),
            Self::CannotSetField { .. } => (21, "field does not exist"),
            Self::NormalizeZeroVector { .. } => (22, "cannot normalize zero vector"),
            Self::InvalidComparison(_, _) => (23, "invalid comparison"),
            Self::ExpectedInteger(_) => (24, "expected integer"),
            Self::IndexOutOfBounds { .. } => (25, "index out of bounds"),
            Self::CannotIndex(_) => (26, "cannot index this type"),
            Self::CannotIterate(_) => (27, "cannot iterate over this type"),

            Self::AstErrorNode => (99, "syntax error prevents evaluation"),
            Self::Internal(_) => (100, "internal error"),
            Self::User(msg) => (0, msg.as_str()),
            Self::Assert(msg) => (0, msg.as_str()),
            Self::AssertCompare(_, _, msg) => (0, msg.as_str()),

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

fn there_should_be_min_max_msg(min: usize, max: usize) -> String {
    if min == max {
        format!("there should be exactly {min}")
    } else {
        format!("there should be between {min} and {max} (inclusive)")
    }
}

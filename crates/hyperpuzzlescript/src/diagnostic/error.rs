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

/// Error or warning, with primary span and traceback.
#[derive(Debug, Clone)]
pub struct FullDiagnostic {
    /// Primary span.
    ///
    /// `msg` may contain more spans.
    pub span: Span,
    /// Caller spans.
    pub traceback: Vec<TracebackLine>,
    /// Error message.
    pub msg: DiagMsg,
}
impl FullDiagnostic {
    /// Adds a caller span to the error message.
    pub fn at_caller(mut self, traceback_line: TracebackLine) -> FullDiagnostic {
        self.traceback.push(traceback_line);
        self
    }

    pub fn kind(&self) -> ariadne::ReportKind<'_> {
        self.msg.overview().0
    }

    /// Returns the error as a string with ANSI escape codes.
    pub fn to_string(&self, mut files: impl ariadne::Cache<FileId>) -> String {
        let (kind, msg) = self.msg.overview();
        let report_builder = ReportBuilder::new(kind, msg, self.span);

        match &self.msg {
            DiagMsg::LexError(e) => report_builder.main_label(e.reason()).labels(
                e.contexts()
                    .map(|(pat, sp)| ((self.span.context, *sp), pat)),
            ),
            DiagMsg::ParseError(e) => report_builder
                .main_label(e.reason())
                .labels(e.contexts().map(|(pat, sp)| (sp, pat))),
            DiagMsg::Unimplemented(_) => {
                report_builder.main_label("this feature isn't implemented yet")
            }
            DiagMsg::TypeError { expected, got } => {
                report_builder.main_label(format!("expected \x02{expected}\x03, got \x02{got}\x03"))
            }
            DiagMsg::Immut { reason } => {
                report_builder.main_label(format!("this cannot be modified because {reason}"))
            }
            DiagMsg::ExpectedType { ast_node_kind } => report_builder
                .main_label(format!("this is a \x02{ast_node_kind}\x03"))
                .help("try a type like `Str` or `List[Num]`"),
            DiagMsg::ExpectedCollectionType { .. } => report_builder
                .main_label(format!("this is not a collection type"))
                .help("try a collection type like `List` or `Map`"),
            DiagMsg::FnOverloadConflict {
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
            DiagMsg::CannotAssignToExpr { kind } => {
                report_builder.main_label(format!("\x02{kind}\x03 is not assignable"))
            }
            DiagMsg::Undefined => report_builder
                .main_label("this is undefined")
                .help("it may be defined somewhere, but isn't accessible from here")
                .help("try assigning `null` to define the variable in an outer scope"),
            DiagMsg::UnknownType => report_builder
                .main_label("unknown type")
                .help("try using a type name like `Num` or `Str`"),
            DiagMsg::WrongNumberOfIndices {
                obj_span,
                count,
                min,
                max,
            } => report_builder
                .main_label(format!("found {count} index value(s)"))
                .label(obj_span, "when indexing this")
                .help(there_should_be_min_max_msg(*min, *max)),
            DiagMsg::WrongNumberOfLoopVars {
                iter_span,
                count,
                min,
                max,
            } => report_builder
                .main_label(format!("found {count} loop variable(s)"))
                .label(iter_span, "when iterating over this")
                .help(there_should_be_min_max_msg(*min, *max)),
            DiagMsg::UnsupportedOperator => report_builder
                .main_label("this operator is not supported")
                .help("contact the developer if you have a use case for it"),
            DiagMsg::NoFnWithName => report_builder.main_label("no function with this name"),
            DiagMsg::BadArgTypes {
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
            DiagMsg::AmbiguousFnCall {
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
            DiagMsg::ExpectedMapKey => report_builder
                .main_label("expected map key")
                .help("convert a value to a string to use it as a map key: `\"${my_value}\"`"),
            DiagMsg::BreakOutsideLoop | DiagMsg::ContinueOutsideLoop => {
                report_builder.main_label("not in a loop")
            }
            DiagMsg::NoField { obj } => report_builder
                .main_label("this field does not exist")
                .label(obj, "on this object"),
            DiagMsg::CannotSetField { obj } => report_builder
                .main_label("cannot set this field")
                .label(obj, "on this object"),
            DiagMsg::NormalizeZeroVector => report_builder.main_label("vector is zero"),
            DiagMsg::InvalidComparison(ty1, ty2) => report_builder
                .main_label("this comparison operator is unsupported on those types")
                .label_type(ty1)
                .label_type(ty2),
            DiagMsg::ExpectedInteger(n) => report_builder.main_label(n),
            DiagMsg::IndexOutOfBounds { got, bounds } => {
                report_builder.main_label(got).note(match bounds {
                    Some((min, max)) => {
                        format!("expected integer between {min} and {max} (inclusive)")
                    }
                    None => format!("collection is empty"),
                })
            }
            DiagMsg::CannotIndex(ty) => {
                report_builder.main_label(format!("this is a \x02{ty}\x03"))
            }
            DiagMsg::CannotIterate(ty) => {
                report_builder.main_label(format!("this is a \x02{ty}\x03"))
            }

            DiagMsg::AstErrorNode => report_builder.help("see earlier errors for a syntax error"),
            DiagMsg::Internal(msg) => report_builder.main_label(msg),
            DiagMsg::UserError(_) => report_builder.main_label("error reported here"),
            DiagMsg::UserWarning(_) => report_builder.main_label("warning reported here"),
            DiagMsg::Assert(_) => report_builder.main_label("error reported here"),
            DiagMsg::AssertCompare(l, r, _) => report_builder.label_values([&**l, &**r]),

            DiagMsg::Return(_) | DiagMsg::Break | DiagMsg::Continue => {
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
            DiagMsg::Return(ret_val) => Ok(std::mem::take(ret_val)),
            DiagMsg::Break => Err(DiagMsg::BreakOutsideLoop.at(self.span)),
            DiagMsg::Continue => Err(DiagMsg::ContinueOutsideLoop.at(self.span)),
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

/// Error or warning, without a primary span or traceback.
#[derive(Debug, Clone)]
#[non_exhaustive]
#[allow(missing_docs)]
pub enum DiagMsg {
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
    UserError(EcoString),
    UserWarning(EcoString),
    Assert(EcoString),
    AssertCompare(Box<Value>, Box<Value>, EcoString),

    /// Not actually an error; used for ordinary return statements.
    Return(Box<Value>),
    /// Not actually an error; used for ordinary break statements.
    Break,
    /// Not actually an error; used for ordinary continue statements.
    Continue,
}
impl DiagMsg {
    /// Adds a primary span to the error.
    pub fn at(self, span: impl Into<Span>) -> FullDiagnostic {
        FullDiagnostic {
            span: span.into(),
            traceback: vec![],
            msg: self,
        }
    }

    /// Reports an error, using `FnDebugInfo` instead of a `Span`.
    ///
    /// This produces slightly error messages on built-in functions, and panics
    /// in debug mode.
    pub fn debug_at(self, debug: FnDebugInfo) -> FullDiagnostic {
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

    pub fn overview(&self) -> (ariadne::ReportKind<'static>, &str) {
        use ariadne::ReportKind::{Error as E, Warning as W};
        match self {
            Self::LexError(_) | Self::ParseError(_) => (E, "syntax error"),
            Self::Unimplemented(_) => (E, "unimplemented"),
            Self::TypeError { .. } => (E, "type error"),
            Self::Immut { .. } => (E, "immutable value"),
            Self::ExpectedType { .. } => (E, "expected type"),
            Self::ExpectedCollectionType { .. } => (E, "expected collection type"),
            Self::FnOverloadConflict { .. } => (E, "conflicting function overload"),
            Self::CannotAssignToExpr { .. } => (E, "non-assignable expression"),
            Self::Undefined => (E, "undefined"),
            Self::UnknownType => (E, "unknown type"),
            Self::WrongNumberOfIndices { .. } => (E, "wrong number of indices"),
            Self::WrongNumberOfLoopVars { .. } => (E, "wrnog number of loop variables"),
            Self::UnsupportedOperator => (E, "unsupported operator"),
            Self::NoFnWithName => (E, "no function with name"),
            Self::BadArgTypes { .. } => (E, "bad argument types"),
            Self::AmbiguousFnCall { .. } => (E, "ambiguous function call"),
            Self::ExpectedMapKey => (E, "expected map key"),
            Self::BreakOutsideLoop => (E, "'break' used outside loop"),
            Self::ContinueOutsideLoop => (E, "'continue' used outside loop"),
            Self::NoField { .. } => (E, "field does not exist"),
            Self::CannotSetField { .. } => (E, "field does not exist"),
            Self::NormalizeZeroVector { .. } => (E, "cannot normalize zero vector"),
            Self::InvalidComparison(_, _) => (E, "invalid comparison"),
            Self::ExpectedInteger(_) => (E, "expected integer"),
            Self::IndexOutOfBounds { .. } => (E, "index out of bounds"),
            Self::CannotIndex(_) => (E, "cannot index this type"),
            Self::CannotIterate(_) => (E, "cannot iterate over this type"),

            Self::AstErrorNode => (E, "syntax error prevents evaluation"),
            Self::Internal(_) => (E, "internal error"),
            Self::UserError(msg) => (E, msg.as_str()),
            Self::UserWarning(msg) => (W, msg.as_str()),
            Self::Assert(msg) => (E, msg.as_str()),
            Self::AssertCompare(_, _, msg) => (E, msg.as_str()),

            Self::Return(_) | Self::Break | Self::Continue => (E, "internal error"),
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

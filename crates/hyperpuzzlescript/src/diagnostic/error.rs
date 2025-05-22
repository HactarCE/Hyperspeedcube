use arcstr::Substr;
use ecow::EcoString;
use itertools::Itertools;

use super::{FullDiagnostic, ReportBuilder};
use crate::{FnType, Span, Spanned, Type, Value};

/// Error message, without traceback information.
#[derive(thiserror::Error, Debug, Clone)]
#[non_exhaustive]
#[allow(missing_docs)]
pub enum Error {
    #[error("syntax error")]
    SyntaxError {
        reason: String,
        contexts: Vec<Spanned<String>>,
    },
    #[error("unimplemented")]
    Unimplemented(&'static str),
    #[error("type error")]
    TypeError { expected: Type, got: Type },
    #[error("immutable value")]
    Immut { reason: ImmutReason },
    #[error("expected type")]
    ExpectedType { got_ast_node_kind: &'static str },
    #[error("expected identifier, assignment, or function definition")]
    ExpectedExportable { got_ast_node_kind: &'static str },
    #[error("expected identifier")]
    ExpectedExportableVar { got_ast_node_kind: &'static str },
    #[error("cannot export with compound assignment")]
    CompoundAssignmentNotAllowed,
    #[error("expected collection type")]
    ExpectedCollectionType,
    #[error("conflicting function overload")]
    FnOverloadConflict {
        new_ty: Box<FnType>,
        old_ty: Box<FnType>,
        old_span: Option<Span>,
    },
    #[error("non-assignable expression")]
    CannotAssignToExpr { kind: &'static str },
    #[error("undefined")]
    Undefined,
    #[error("unknown type")]
    UnknownType,
    #[error("wrong number of indices")]
    WrongNumberOfIndices {
        obj_span: Span,
        count: usize,
        min: usize,
        max: usize,
    },
    #[error("wrnog number of loop variables")]
    WrongNumberOfLoopVars {
        iter_span: Span,
        count: usize,
        min: usize,
        max: usize,
    },
    #[error("unsupported operator")]
    UnsupportedOperator,
    #[error("no function with name")]
    NoFnWithName,
    #[error("bad argument types")]
    BadArgTypes {
        arg_types: Vec<Spanned<Type>>,
        overloads: Vec<FnType>,
    },
    #[error("ambiguous function call")]
    AmbiguousFnCall {
        arg_types: Vec<Spanned<Type>>,
        overloads: Vec<FnType>,
    },
    #[error("expected map key")]
    ExpectedMapKey,
    #[error("'break' used outside loop")]
    BreakOutsideLoop,
    #[error("'continue' used outside loop")]
    ContinueOutsideLoop,
    #[error("field does not exist")]
    NoField { obj: Span },
    #[error("field does not exist")]
    CannotSetField { obj: Span },
    #[error("cannot normalize zero vector")]
    NormalizeZeroVector,
    #[error("invalid comparison")]
    InvalidComparison(Box<Spanned<Type>>, Box<Spanned<Type>>),
    #[error("expected integer")]
    ExpectedInteger(f64),
    #[error("index out of bounds")]
    IndexOutOfBounds {
        got: i64,
        bounds: Option<(i64, i64)>,
    },
    #[error("cannot index this type")]
    CannotIndex(Type),
    #[error("cannot iterate over this type")]
    CannotIterate(Type),
    #[error("domain error")]
    NaN,
    #[error("infinity")]
    Infinity,

    // TODO: make sure we're handling AST error node properly
    #[error("syntax error prevents evaluation")]
    AstErrorNode,
    #[error("internal error")]
    Internal(&'static str),
    #[error("{0}")]
    User(EcoString),
    #[error("{0}")]
    Assert(EcoString),
    #[error("{2}")]
    AssertCompare(Box<Value>, Box<Value>, EcoString),

    /// Not actually an error; used for ordinary return statements.
    #[error("internal")]
    Return(Box<Value>),
    /// Not actually an error; used for ordinary break statements.
    #[error("internal")]
    Break,
    /// Not actually an error; used for ordinary continue statements.
    #[error("internal")]
    Continue,
}
impl Error {
    /// Adds a primary span to the error.
    pub fn at(self, span: impl Into<Span>) -> FullDiagnostic {
        FullDiagnostic {
            msg: self.into(),
            span: span.into(),
            traceback: vec![],
        }
    }

    pub(super) fn report(&self, report_builder: ReportBuilder) -> ReportBuilder {
        match self {
            Self::SyntaxError { reason, contexts } => report_builder
                .main_label(reason)
                .labels(contexts.iter().map(|(msg, span)| (*span, msg))),
            Self::Unimplemented(_) => {
                report_builder.main_label("this feature isn't implemented yet")
            }
            Self::TypeError { expected, got } => {
                report_builder.main_label(format!("expected \x02{expected}\x03, got \x02{got}\x03"))
            }
            Self::Immut { reason } => {
                report_builder.main_label(format!("this cannot be modified because {reason}"))
            }
            Self::ExpectedType { got_ast_node_kind } => report_builder
                .main_label(format!("this is a \x02{got_ast_node_kind}\x03"))
                .help("try a type like `Str` or `List[Num]`"),
            Self::ExpectedExportable { got_ast_node_kind }
            | Self::ExpectedExportableVar { got_ast_node_kind } => {
                report_builder.main_label(format!("this is a \x02{got_ast_node_kind}\x03"))
            }
            Self::CompoundAssignmentNotAllowed => report_builder
                .main_label("compound assignment operator")
                .note("compound assignment operators are not allowed in `export` statements")
                .help("modify the variable first, then export it on another line"),
            Self::ExpectedCollectionType { .. } => report_builder
                .main_label(format!("this is not a collection type"))
                .help("try a collection type like `List` or `Map`"),
            Self::FnOverloadConflict {
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
            Self::CannotAssignToExpr { kind } => {
                report_builder.main_label(format!("\x02{kind}\x03 is not assignable"))
            }
            Self::Undefined => report_builder
                .main_label("this is undefined")
                .help("it may be defined somewhere, but isn't accessible from here")
                .help("try assigning `null` to define the variable in an outer scope"),
            Self::UnknownType => report_builder
                .main_label("unknown type")
                .help("try using a type name like `Num` or `Str`"),
            Self::WrongNumberOfIndices {
                obj_span,
                count,
                min,
                max,
            } => report_builder
                .main_label(format!("found {count} index value(s)"))
                .label(obj_span, "when indexing this")
                .help(there_should_be_min_max_msg(*min, *max)),
            Self::WrongNumberOfLoopVars {
                iter_span,
                count,
                min,
                max,
            } => report_builder
                .main_label(format!("found {count} loop variable(s)"))
                .label(iter_span, "when iterating over this")
                .help(there_should_be_min_max_msg(*min, *max)),
            Self::UnsupportedOperator => report_builder
                .main_label("this operator is not supported")
                .help("contact the developer if you have a use case for it"),
            Self::NoFnWithName => report_builder.main_label("no function with this name"),
            Self::BadArgTypes {
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
            Self::AmbiguousFnCall {
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
            Self::ExpectedMapKey => report_builder
                .main_label("expected map key")
                .help("convert a value to a string to use it as a map key: `\"${my_value}\"`"),
            Self::BreakOutsideLoop | Self::ContinueOutsideLoop => {
                report_builder.main_label("not in a loop")
            }
            Self::NoField { obj } => report_builder
                .main_label("this field does not exist")
                .label(obj, "on this object"),
            Self::CannotSetField { obj } => report_builder
                .main_label("cannot set this field")
                .label(obj, "on this object"),
            Self::NormalizeZeroVector => report_builder.main_label("vector is zero"),
            Self::InvalidComparison(ty1, ty2) => report_builder
                .main_label("this comparison operator is unsupported on those types")
                .label_type(ty1)
                .label_type(ty2),
            Self::ExpectedInteger(n) => report_builder.main_label(n),
            Self::IndexOutOfBounds { got, bounds } => {
                report_builder.main_label(got).note(match bounds {
                    Some((min, max)) => {
                        format!("expected integer between {min} and {max} (inclusive)")
                    }
                    None => format!("collection is empty"),
                })
            }
            Self::CannotIndex(ty) => report_builder.main_label(format!("this is a \x02{ty}\x03")),
            Self::CannotIterate(ty) => report_builder.main_label(format!("this is a \x02{ty}\x03")),
            Self::NaN => report_builder
                .main_label("not a number")
                .help("check for division by zero or other invalid operation"),
            Self::Infinity => report_builder
                .main_label("infinity is not allowed here")
                .help("check for division by zero"),

            Self::AstErrorNode => report_builder.help("see earlier errors for a syntax error"),
            Self::Internal(msg) => report_builder.main_label(msg),
            Self::User(_) => report_builder.main_label("error reported here"),
            Self::Assert(_) => report_builder.main_label("error reported here"),
            Self::AssertCompare(l, r, _) => report_builder.label_values([&**l, &**r]),

            Self::Return(_) | Self::Break | Self::Continue => {
                report_builder.main_label("leaked control flow")
            }
        }
    }
}

/// Reason for a variable being immutable.
#[derive(thiserror::Error, Debug, Clone)]
#[non_exhaustive]
#[allow(missing_docs)]
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

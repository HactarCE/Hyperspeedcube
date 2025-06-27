use std::{borrow::Cow, fmt};

use arcstr::Substr;
use ecow::{EcoString, eco_format};
use itertools::Itertools;

use super::{FullDiagnostic, ReportBuilder};
use crate::{
    FILE_EXTENSION, FnType, INDEX_FILE_NAME, Key, Span, Spanned, Type, Value, ValueData, ast,
};

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
    #[error("list length error")]
    ListLengthError { expected: usize, got: usize },
    #[error("immutable value")]
    Immut { reason: ImmutReason },
    #[error("expected type")]
    ExpectedType { got_ast_node_kind: &'static str },
    #[error("expected identifier, assignment, or function definition")]
    ExpectedExportable { got_ast_node_kind: &'static str },
    #[error("expected identifier")]
    ExpectedExportableVar { got_ast_node_kind: &'static str },
    #[error("return statement after export")]
    ReturnAfterExport { export_spans: Vec<Span> },
    #[error("expected collection type")]
    ExpectedCollectionType { got_type: Type },
    #[error("conflicting function overload")]
    FnOverloadConflict {
        new_ty: Box<FnType>,
        old_ty: Box<FnType>,
        old_span: Option<Span>,
    },
    #[error("non-assignable expression")]
    CannotAssignToExpr { kind: &'static str },
    #[error("non-destructurable expression in pattern")]
    CannotDestructureToExpr { kind: &'static str },
    #[error("splat before end in pattern")]
    SplatBeforeEndInPattern { pattern_span: Span },
    #[error("list length mismatch")]
    ListLengthMismatchInPattern {
        pattern_span: Span,
        pattern_len: usize,
        allow_excess: bool,
        value_len: usize,
    },
    #[error("map contains extra keys not present in pattern")]
    UnusedMapKeysInPattern {
        pattern_span: Span,
        keys: Vec<Spanned<Key>>,
    },
    #[error("duplicate map key")]
    DuplicateMapKey { previous_span: Span },
    #[error("duplicate parameter name")]
    DuplicateFnParamName { previous_span: Span },
    #[error("unused function arguments")]
    UnusedPositionalFnArgs,
    #[error("unused function arguments")]
    UnusedNamedFnArgs { names: Vec<Spanned<Key>> },
    #[error("default value is not allowed for positional parameter")]
    DefaultPositionalParamValue,
    #[error("duplicate 'end of sequence' indicator in function parameters")]
    DuplicateFnParamSeqEnd { previous_span: Span },
    #[error("splat before end in function parameters")]
    FnParamSplatBeforeEnd,
    #[error("splat before end in function arguments")]
    FnArgSplatBeforeEnd,
    #[error("missing required positional parameter")]
    MissingRequiredPositionalParameter { ty: Type },
    #[error("missing required named parameter")]
    MissingRequiredNamedParameter { name: Key, ty: Type },
    #[error("positional parameter after named parameter")]
    PositionalParamAfterNamedParam,
    #[error("unused map keys")]
    UnusedMapKeys { keys: Vec<Spanned<Key>> },
    #[error("missing required key")]
    MissingRequiredMapKey { key: Key },
    #[error("undefined")]
    Undefined,
    #[error("undefined in map")]
    UndefinedIn(Span),
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
    #[error("missing map value")]
    MissingMapValue,
    #[error("'break' used outside loop")]
    BreakOutsideLoop,
    #[error("'continue' used outside loop")]
    ContinueOutsideLoop,
    #[error("field does not exist")]
    NoField(Spanned<Type>),
    #[error("field does not exist")]
    CannotSetField(Spanned<Type>),
    #[error("bad argument")]
    BadArgument { value: String, note: Option<String> },
    #[error("invalid comparison")]
    InvalidComparison(Box<Spanned<Type>>, Box<Spanned<Type>>),
    #[error("expected integer")]
    ExpectedInteger(f64),
    #[error("expected nonnegative integer")]
    ExpectedNonnegativeInteger(f64),
    #[error("expected nonnegative integer < 256")]
    ExpectedSmallNonnegativeInteger(f64),
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
    #[error("range exceeds maximum length")]
    RangeTooBig { len: usize },
    #[error("file not found")]
    ModuleNotFound { path: String, is_relative: bool },
    #[error("path accesses beyond root")]
    BeyondRoot,

    #[error("cannot assign to special value")]
    CannotAssignToSpecialVar(ast::SpecialVar),
    #[error("number of dimensions is undefined")]
    NoNdim,
    #[error("symmetry is undefined")]
    NoSym,

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
    #[error("internal: leaked 'return'")]
    Return(Box<Value>),
    /// Not actually an error; used for ordinary break statements.
    #[error("internal: leaked 'break'")]
    Break,
    /// Not actually an error; used for ordinary continue statements.
    #[error("internal: leaked 'continue'")]
    Continue,
    /// Imported file failed to load; do not report an error here.
    #[error("internal: leaked import error")]
    SilentImportError,
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

    pub(crate) fn bad_arg(v: impl Into<ValueData>, note: Option<impl ToString>) -> Self {
        Self::BadArgument {
            value: v.into().repr(),
            note: note.map(|n| n.to_string()),
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
            Self::ListLengthError { expected, got } => report_builder.main_label(format!(
                "expected \x02{expected}\x03 elements, got \x02{got}\x03 elements"
            )),
            Self::Immut { reason } => {
                report_builder.main_label(format!("this cannot be modified because {reason}"))
            }
            Self::ExpectedType { got_ast_node_kind } => report_builder
                .main_label(format!("\x02this\x03 is a \x02{got_ast_node_kind}\x03"))
                .help("try a type like `Str` or `List[Num]`"),
            Self::ExpectedExportable { got_ast_node_kind }
            | Self::ExpectedExportableVar { got_ast_node_kind } => {
                report_builder.main_label(format!("this is a \x02{got_ast_node_kind}\x03"))
            }
            Self::ReturnAfterExport { export_spans } => report_builder
                .main_label("before \x02this return statement\x03")
                .labels(
                    export_spans
                        .iter()
                        .map(|span| (span, "\x02this value\x03 was previously exported")),
                )
                .note("returing a value and exporting values are mutually exclusive"),
            Self::ExpectedCollectionType { got_type } => report_builder
                .main_label(format!("\x02{got_type}\x03 is not a collection type"))
                .help("try a collection type like `List`"),
            Self::FnOverloadConflict {
                new_ty,
                old_ty,
                old_span,
            } => report_builder
                .main_label(format!("\x02new overload\x03 has type \x02{new_ty}\x03"))
                .label_or_note(
                    *old_span,
                    format!("\x02previous overload\x03 has type \x02{old_ty}\x03"),
                )
                .note(
                    "some type overlaps may be non-obvious; for example:\n\
                     • `List[Str]` and `List[Num]` overlap on the empty list\n\
                     • `Num`, `Int`, and `Nat` all overlap on non-negative integers",
                ),
            Self::CannotAssignToExpr { kind } => {
                report_builder.main_label(format!("\x02{kind}\x03 is not assignable"))
            }
            Self::CannotDestructureToExpr { kind } => {
                report_builder.main_label(format!("\x02{kind}\x03 is not destructurable"))
            }
            Self::SplatBeforeEndInPattern { pattern_span } => report_builder
                .main_label("\x02splat\x03 is only allowed at end")
                .label(pattern_span, "in \x02this pattern\x03")
                .help("splat pattern gathers unused entries"),
            Self::ListLengthMismatchInPattern {
                pattern_span,
                pattern_len,
                allow_excess,
                value_len,
            } => report_builder
                .main_label(format!("\x02this value\x03 has length \x02{value_len}\x03"))
                .label(
                    pattern_span,
                    format!(
                        "\x02this pattern\x03 expects length \x02{}{pattern_len}\x03",
                        if *allow_excess { "at least " } else { "" },
                    ),
                ),
            Self::UnusedMapKeysInPattern { pattern_span, keys } => report_builder
                .main_label("from \x02this map\x03")
                .label(pattern_span, "in \x02this pattern\x03")
                .labels(
                    keys.iter()
                        .map(|(k, span)| (span, format!("unused entry with key {k:?}"))),
                )
                .help("add `**extra` to the pattern to put unused keys into a new map"),
            Self::DuplicateMapKey { previous_span } => report_builder
                .main_label("duplicate key")
                .label(previous_span, "previous occurrence"),
            Self::DuplicateFnParamName { previous_span } => report_builder
                .main_label("duplicate parameter name")
                .label(previous_span, "previous parameter with same name"),
            Self::UnusedPositionalFnArgs => report_builder.main_label("first unused argument"),
            Self::UnusedNamedFnArgs { names } => report_builder
                .main_label("when calling \x02this function\x03")
                .labels(
                    names
                        .iter()
                        .map(|(k, span)| (span, format!("unused argument with key {k:?}"))),
                ),
            Self::DefaultPositionalParamValue => report_builder
                .main_label("for \x02this parameter\x03")
                .help("add `*,` before the parameter name to make it a named parameter"),
            Self::DuplicateFnParamSeqEnd { previous_span } => report_builder
                .main_label("end of sequence parameters")
                .label(*previous_span, "previous end of sequence parameters"),
            Self::FnParamSplatBeforeEnd | Self::FnArgSplatBeforeEnd => report_builder
                .main_label("\x02splat\x03 is only allowed at end")
                .help("splat pattern gathers unused named parameters"),
            Self::MissingRequiredPositionalParameter { ty } => report_builder
                .main_label("missing required positional parameter")
                .note(format!("expected positional parameter with type `{ty}`")),
            Self::MissingRequiredNamedParameter { name, ty } => report_builder
                .main_label(format!("missing required named parameter `\x02{name}\x03`"))
                .help(format!("add `{name} = ...` at the call site"))
                .note(format!("expected named parameter with type `{ty}`")),
            Self::PositionalParamAfterNamedParam => report_builder
                .main_label("positional parameter occurs after named parameter")
                .note("all positional parameters must come before the first named parameter"),
            Self::UnusedMapKeys { keys } => {
                report_builder.main_label("in \x02this map\x03").labels(
                    keys.iter()
                        .map(|(k, span)| (span, format!("unused entry with key {k:?}"))),
                )
            }
            Self::MissingRequiredMapKey { key } => {
                report_builder.main_label(format!("expected \x02this map\x03 to have {key:?}"))
            }
            Self::Undefined => report_builder
                .main_label("\x02this name\x03 is not defined")
                .help("it may be defined somewhere, but isn't accessible from here")
                .help("try assigning `null` to define the variable in an outer scope"),
            Self::UndefinedIn(map_span) => report_builder
                .main_label("\x02this name\x03 is not defined")
                .label(map_span, "in \x02this map\x03"),
            Self::UnknownType => report_builder
                .main_label("unknown type")
                .help("try using a type name like `Num` or `Str`"),
            Self::WrongNumberOfIndices {
                obj_span,
                count,
                min,
                max,
            } => report_builder
                .main_label(format!("found \x02{count} index value(s)\x03"))
                .label(obj_span, "when indexing \x02this\x03")
                .help(there_should_be_min_max_msg(*min, *max)),
            Self::WrongNumberOfLoopVars {
                iter_span,
                count,
                min,
                max,
            } => report_builder
                .main_label(format!("found \x02{count} loop variable(s)\x03"))
                .label(iter_span, "when iterating over \x02this\x03")
                .help(there_should_be_min_max_msg(*min, *max)),
            Self::UnsupportedOperator => report_builder
                .main_label("\x02this operator\x03 is not supported")
                .help("contact the developer if you have a use case for it"),
            Self::NoFnWithName => report_builder.main_label("no function with this name"),
            Self::BadArgTypes {
                arg_types,
                overloads,
            } => report_builder
                .main_label({
                    let types = arg_types.iter().map(|(ty, _)| ty).join(", ");
                    format!("no overload for \x02this function\x03 matches Fn({types})")
                })
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
                .main_label("for \x02this function\x03")
                .label_types(arg_types)
                .notes(
                    overloads
                        .iter()
                        .map(|candidate| format!("could be {candidate}")),
                ),
            Self::ExpectedMapKey => report_builder
                .main_label("expected map key")
                .help("convert a value to a string to use it as a map key: `\"${my_value}\"`"),
            Self::MissingMapValue => report_builder
                .main_label("missing map value")
                .help("add `= value`"),
            Self::BreakOutsideLoop | Self::ContinueOutsideLoop => {
                report_builder.main_label("not in a loop")
            }
            Self::NoField((obj_ty, obj_span)) => report_builder
                .main_label("\x02this field\x03 does not exist")
                .label(obj_span, format!("on this object of type \x02{obj_ty}\x03")),
            Self::CannotSetField((obj_ty, obj_span)) => report_builder
                .main_label("cannot set \x02this field\x03")
                .label(obj_span, format!("on this object of type \x02{obj_ty}\x03")),
            Self::BadArgument { value, note } => report_builder
                .main_label(format!("bad argument: \x02{value}\x03"))
                .notes(note),
            Self::InvalidComparison(ty1, ty2) => report_builder
                .main_label("\x02this comparison operator\x03 is unsupported on these types")
                .label_type(ty1)
                .label_type(ty2),
            Self::ExpectedInteger(n)
            | Self::ExpectedNonnegativeInteger(n)
            | Self::ExpectedSmallNonnegativeInteger(n) => report_builder.main_label(n),

            Self::IndexOutOfBounds { got, bounds } => {
                report_builder.main_label(got).note(match bounds {
                    Some((min, max)) => {
                        format!("expected integer between {min} and {max} (inclusive)")
                    }
                    None => "collection is empty".to_string(),
                })
            }
            Self::CannotIndex(ty) | Self::CannotIterate(ty) => {
                report_builder.main_label(format!("this is a \x02{ty}\x03"))
            }
            Self::NaN => report_builder
                .main_label("not a number")
                .help("check for division by zero or other invalid operation"),
            Self::Infinity => report_builder
                .main_label("infinity is not allowed here")
                .help("check for division by zero"),
            Self::RangeTooBig { len } => report_builder
                .main_label("\x02this range\x03 is too big")
                .note(format!("max range length is {len}")),
            Self::ModuleNotFound { path, is_relative } => report_builder
                .main_label(format!("failed to find module \x02{path}\x03"))
                .help(if *is_relative {
                    "this is a relative path; remove \
                     the first `/` to make it absolute"
                } else {
                    "this is an absolute path; add a `/` after \
                     `@` to make it relative to the current file"
                })
                .help(format!(
                    "expected at one of these locations:\n\
                     \x02{path}.{FILE_EXTENSION}\x03\n\
                     \x02{path}/{INDEX_FILE_NAME}.{FILE_EXTENSION}\x03"
                )),
            Self::BeyondRoot => report_builder
                .main_label("\x02this relative path\x03 reaches beyond the root directory")
                .help("try removing some `^`s"),

            Self::CannotAssignToSpecialVar(var) => report_builder
                .main_label("cannot assign to \x02this\x03")
                .help(format!("use `with {var} = ... {{ ... }}`")),
            Self::NoNdim => report_builder.main_label("this requires `#ndim` to be defined"),
            Self::NoSym => report_builder.main_label("this requires `#sym` to be defined"),

            Self::AstErrorNode => report_builder.help("see earlier errors for a syntax error"),
            Self::Internal(msg) => report_builder.main_label(msg),
            Self::User(_) | Self::Assert(_) => report_builder.main_label("error reported here"),
            Self::AssertCompare(l, r, _) => report_builder.label_values([&**l, &**r]),

            Self::Return(_) | Self::Break | Self::Continue | Self::SilentImportError => {
                report_builder.main_label("error reported here")
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

impl From<eyre::Report> for Error {
    fn from(e: eyre::Report) -> Self {
        Error::User(eco_format!("{e:#}"))
    }
}

impl<'a> From<Cow<'a, str>> for Error {
    fn from(value: Cow<'a, str>) -> Self {
        Error::User(value.into())
    }
}

impl<'a> From<&'a str> for Error {
    fn from(value: &'a str) -> Self {
        Error::User(value.into())
    }
}

/// Extension trait for [`Error`]-compatible errors and results.
pub trait ErrorExt {
    /// [`FullDiagnostic`] or [`crate::Result<T>`].
    type Output;

    /// Attaches a span to the error.
    fn at(self, span: Span) -> Self::Output;
}
impl<E: Into<Error>> ErrorExt for E {
    type Output = FullDiagnostic;

    fn at(self, span: Span) -> Self::Output {
        let e: Error = self.into();
        e.at(span)
    }
}
impl<T, E: fmt::Display> ErrorExt for Result<T, E> {
    type Output = Result<T, FullDiagnostic>;

    fn at(self, span: Span) -> Self::Output {
        // Alternate formatting includes more detail for `eyre::Report`s.
        self.map_err(|e| Error::User(eco_format!("{e:#}")).at(span))
    }
}

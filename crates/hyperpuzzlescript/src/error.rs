use std::{fmt, ops::Range};

use arcstr::Substr;
use itertools::Itertools;
use thiserror::Error;

use crate::{
    FileIndex, Span,
    lexer::{LexError, Token},
    parser::ParseError,
    span_to_range,
    ty::{FnType, Type},
};

// pub type Error = ariadne::Report<'static>;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Error {
    // #[error("{0}")]
    LexError(FileIndex, LexError<'static>),
    ParseError(ParseError<'static>),

    // #[error("{0:?} is not yet implemented; contact the developers if you have a use case")]
    Unimplemented(&'static str),

    // #[error("expected {expected}; got {got}")]
    TypeError { expected: Type, got: Type },

    // #[error("cannot modify `{name}`: {reason}")]
    Immut { name: Substr, reason: ImmutReason },

    // #[error("overlapping function implementations: {ty1} and {ty2}")]
    FnOverloadConflict { ty1: Box<FnType>, ty2: Box<FnType> },
}
impl Error {
    pub fn report(&self) -> ariadne::Report<'static, AriadneSpan> {
        let mut next_color = color_generator();

        match self {
            Error::LexError(file, e) => {
                ariadne::Report::build(ariadne::ReportKind::Error, (*file, *e.span()).into())
                    .with_label(ariadne_label(next_color(), (*file, *e.span()), e.reason()))
                    .with_labels(
                        e.contexts()
                            .map(|(pat, span)| ariadne_label(next_color(), (*file, *span), pat)),
                    )
                    .with_code(1)
                    .with_message("syntax error")
                    .finish()
            }

            Error::ParseError(e) => {
                ariadne::Report::build(ariadne::ReportKind::Error, AriadneSpan(*e.span()))
                    .with_label(ariadne_label(next_color(), *e.span(), e.reason()))
                    .with_labels(
                        e.contexts()
                            .map(|(pat, span)| ariadne_label(next_color(), *span, pat)),
                    )
                    .with_code(1)
                    .with_message("syntax error")
                    .finish()
            }

            _ => todo!(),
        }
    }
}

fn ariadne_span(s: Span) -> (FileIndex, Range<usize>) {
    (s.context, s.start as usize..s.end as usize)
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct AriadneSpan(Span);
impl ariadne::Span for AriadneSpan {
    type SourceId = FileIndex;

    fn source(&self) -> &Self::SourceId {
        &self.0.context
    }
    fn start(&self) -> usize {
        self.0.start as usize
    }
    fn end(&self) -> usize {
        self.0.end as usize
    }
}
impl From<Span> for AriadneSpan {
    fn from(value: Span) -> Self {
        Self(value)
    }
}
impl From<(FileIndex, chumsky::span::SimpleSpan)> for AriadneSpan {
    fn from((file_index, span): (FileIndex, chumsky::span::SimpleSpan)) -> Self {
        Self(Span {
            start: span.start as u32,
            end: span.end as u32,
            context: file_index,
        })
    }
}

fn color_generator() -> impl FnMut() -> ariadne::Color {
    let mut ariadne_color_generator = ariadne::ColorGenerator::new();
    ariadne_color_generator.next();

    let mut iter = itertools::chain(
        // some nice colors selected from https://www.calmar.ws/vim/256-xterm-24bit-rgb-color-chart.html
        [81, 207, 220, 156, 211, 104, 208, 49, 26, 101].map(ariadne::Color::Fixed),
        std::iter::from_fn(move || Some(ariadne_color_generator.next())),
    );

    move || iter.next().unwrap()
}

fn ariadne_label(
    color: ariadne::Color,
    span: impl Into<AriadneSpan>,
    msg: impl ToString,
) -> ariadne::Label<AriadneSpan> {
    use ariadne::Fmt;

    ariadne::Label::new(span.into())
        .with_message(msg.to_string().fg(color))
        .with_color(color)
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Warning {}

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

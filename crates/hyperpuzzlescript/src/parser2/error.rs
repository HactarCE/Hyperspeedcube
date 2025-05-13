use std::num::ParseFloatError;

use thiserror::Error;

use super::{LexError, Span, Spanned};

pub type ParseResult<T> = Result<T, ParseError>;
pub type ParseError = Spanned<ParseErrorMsg>;

#[derive(Error, Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ParseErrorMsg {
    #[error("{0}")]
    LexError(#[from] LexError),

    #[error("expected {0}")]
    Expected(String),

    #[error("bad number literal: {0}")]
    BadNumber(#[from] ParseFloatError),
    #[error("unknown escape character: {0:?}")]
    BadEscapeChar(char),

    #[error("{0:?} are not yet implemented; contact the developers if you have a use case")]
    Unimplemented(&'static str),
}
impl ParseErrorMsg {
    pub fn at(self, span: impl Into<Span>) -> ParseError {
        Spanned::new(span, self)
    }
}

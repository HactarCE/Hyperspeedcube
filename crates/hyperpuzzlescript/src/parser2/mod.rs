//! Parser that turns a flat list of tokens directly into an AST.

use arcstr::{ArcStr, Substr};
use itertools::Itertools;

#[macro_use]
mod macros;
mod error;
mod lexer;
pub mod rules;

pub use error::{ParseError, ParseErrorMsg, ParseResult};
use lexer::{LexError, StringLiteralSegment, Token};
use rules::SyntaxRule;

use crate::{Span, Spanned, ast};

/// Parses a file.
pub fn parse_file(source: ArcStr) -> ParseResult<Vec<ast::Node>> {
    let tokens: Vec<Spanned<Token>> = lexer::tokenize(&source)
        .map(|Spanned { span, inner }| match inner {
            Ok(token) => Ok(Spanned::new(span, token)),
            Err(e) => Err(ParseErrorMsg::LexError(e).at(span)),
        })
        .try_collect()?;
    let mut p = Parser::new(&source, &tokens);
    p.parse(rules::stmt::BlockContents)
}

/// Token parser used to assemble an AST.
#[derive(Debug, Copy, Clone)]
pub struct Parser<'a> {
    /// Source string.
    pub source: &'a ArcStr,
    /// Tokens to feed.
    pub tokens: &'a [Spanned<Token>],
    /// Index of the "current" token (None = before start).
    pub cursor: Option<usize>,
}
impl<'a> Parser<'a> {
    /// Constructs a parser for a file.
    pub fn new(source: &'a ArcStr, tokens: &'a [Spanned<Token>]) -> Self {
        let mut ret = Self {
            source,
            tokens,
            cursor: None,
        };

        // Skip leading `=`
        ret.next();
        if ret.token_str() != "=" {
            // Oops, no leading `=` so go back
            ret.cursor = None;
        }
        ret
    }

    /// Returns the token at the cursor.
    pub fn current(self) -> &'a Token {
        // IIFE to mimic try_block
        (|| Some(&self.tokens.get(self.cursor?)?.inner))().unwrap_or(&Token::Eof)
    }
    /// Returns the span of the current token. If there is no current token,
    /// returns an empty span at the beginning or end of the input appropriately.
    pub fn span(&self) -> Span {
        if let Some(idx) = self.cursor {
            if let Some(token) = self.tokens.get(idx) {
                token.span
            } else {
                // This is the end of the region; return an empty span at the
                // end of the region.
                Span::empty(self.source.len() as u32)
            }
        } else {
            // This is the beginning of the region; return an empty span at the
            // beginning of the region.
            Span::empty(0)
        }
    }
    /// Returns the source string of the current token. If there is no current
    /// token, returns an empty string.
    pub fn token_str(&self) -> &'a str {
        let Span { start, end } = self.span();
        &self.source[start as usize..end as usize]
    }
    /// Same as [`Self::token_str()`], but returns a [`Substr`] instead.
    pub fn token_substr(&self) -> Substr {
        let Span { start, end } = self.span();
        self.source.substr(start as usize..end as usize)
    }

    /// Moves the cursor forward and then returns the token at the cursor.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> &'a Token {
        // Add 1 or set to zero.
        self.cursor = Some(self.cursor.map(|idx| idx + 1).unwrap_or(0));
        self.current()
    }
    /// Moves the cursor back and then returns the token at the cursor.
    pub fn prev(&mut self) -> &'a Token {
        // Subtract 1 if possible.
        self.cursor = self.cursor.and_then(|idx| idx.checked_sub(1));
        self.current()
    }

    /// Returns the token after the one at the cursor, without mutably moving
    /// the cursor.
    pub fn peek_next(self) -> &'a Token {
        let mut tmp = self;
        tmp.next()
    }

    /// Returns the span of the token after the one at the cursor, without
    /// mutably moving the cursor.
    pub fn peek_next_span(self) -> Span {
        let mut tmp = self;
        tmp.next();
        tmp.span()
    }

    /// Attempts to apply a syntax rule starting at the cursor, returning an
    /// error if it fails. This should only be used when this syntax rule
    /// represents the only valid parse; if there are other options,
    /// `try_parse()` is preferred.
    pub fn parse<R: SyntaxRule>(&mut self, rule: R) -> ParseResult<R::Output> {
        self.try_parse(&rule).unwrap_or_else(|| self.expected(rule))
    }
    /// Applies a syntax rule starting at the cursor, returning `None` if the
    /// syntax rule definitely doesn't match (i.e., its `might_match()`
    /// implementation returned false).
    pub fn try_parse<R: SyntaxRule>(&mut self, rule: R) -> Option<ParseResult<R::Output>> {
        rule.prefix_matches(*self).then(|| {
            let old_state = *self; // Save state.
            let ret = rule.consume_match(self);
            if ret.is_err() {
                // Restore prior state on failure.
                *self = old_state;
            }
            ret
        })
    }

    /// Returns an error describing that `expected` was expected at the current
    /// token.
    pub fn expected<T>(self, expected: impl ToString) -> ParseResult<T> {
        // TODO: when #[feature(never_type)] stabilizes, use that here and
        // return ParseResult<!>.
        Err(self.expected_err(expected))
    }
    /// Returns an error describing that `expected` was expected at the current
    /// token.
    pub fn expected_err(self, expected: impl ToString) -> ParseError {
        ParseErrorMsg::Expected(expected.to_string()).at(self.span())
    }
}

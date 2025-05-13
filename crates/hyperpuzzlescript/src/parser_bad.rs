use std::{iter::Peekable, num::ParseFloatError};

use arcstr::{ArcStr, Substr};
use itertools::Itertools;
use logos::{Lexer, Span, SpannedIter};
use thiserror::Error;

use crate::{
    Spanned,
    ast::{AstNode, AstNodeContents, StringSegment},
    lex::{LexError, StringLiteralSegment, Token},
    merge_spans,
};

type ParseResult<T> = Result<T, ParseError>;

/// User-facing string to represent EOF
const EOF: &'static str = "end of file";

struct Parser<'a> {
    source: &'a ArcStr,
    tokens: &'a [(Token, Span)],
    index: Option<usize>,
}
impl<'a> Parser<'a> {
    fn get(&self, index: Option<usize>) -> (&'a Token, Span) {
        match index {
            Some(i) => match self.tokens.get(i) {
                Some((token, span)) => (token, span.clone()),
                None => {
                    let end = self.source.len();
                    (&Token::Eof, end..end)
                }
            },
            None => (&Token::Eof, 0..0),
        }
    }
    fn current(&self) -> &'a Token {
        self.get(self.index).0
    }
    fn next(&mut self) -> &'a Token {
        match &mut self.index {
            Some(i) if *i < self.tokens.len() => *i += 1,
            Some(_) => (),
            None => self.index = Some(0),
        };
        self.current()
    }

    fn peek_index(&self) -> usize {
        match self.index {
            Some(i) => i + 1,
            None => 0,
        }
    }
    fn peek(&self) -> &Token {
        self.get(Some(self.peek_index())).0
    }
    fn peek_span(&self) -> Span {
        self.get(Some(self.peek_index())).1
    }

    fn span(&self) -> Span {
        match self.index {
            Some(i) => match self.tokens.get(i) {
                Some((_token, span)) => span.clone(),
                None => self.source.len()..self.source.len(),
            },
            None => 0..0,
        }
    }
    fn substr(&self) -> Substr {
        self.source.substr(self.span())
    }
    fn str(&self) -> &str {
        &self.source[self.span()]
    }
    fn next_if(&mut self, expected: Token) -> bool {
        let token = self.peek();
        if *token == expected {
            self.next();
            true
        } else {
            false
        }
    }
    fn expect<R: Rule>(&mut self, rule: R) -> ParseResult<R::Output> {
        if rule.prefix_matches(self.peek()) {
            rule.parse(self)
        } else {
            Err(ParseErrorMsg::expected(R::NAME, token).at(self.span()))
        }
    }

    fn is_eof(&mut self) -> bool {
        self.index.is_some_and(|i| i >= self.tokens.len())
    }

    fn statements(&mut self, top_level: bool) -> ParseResult<Vec<AstNode>> {
        let mut statements = vec![];
        while !self.is_eof() {
            statements.push(self.statement(top_level)?);
        }
        Ok(statements)
    }

    fn fn_def(&mut self) -> ParseResult<AstNode> {
        self.expect(Token::Fn)?;
        let first_ident = self.expect(Token::Ident)?;
        let second_ident = if self.next_if(Token::Period)?.is_some() {
            Some(self.expect(Token::Ident)?)
        } else {
            None
        };
        match (first_ident, second_ident) {
            (type_ident, Some(fn_name)) => Ok(AstNode {
                span: merge_spans(first_ident, second_ident),
                contents,
            }),
            (fn_name, None) => todo!(),
        }
    }

    fn expected(&self, expected: impl Into<&'static str>) -> ParseError {
        ParseErrorMsg::expected(expected, self.current()).at(self.span())
    }
}

trait Rule {
    const NAME: &str;

    type Output;

    fn prefix_matches(&self, token: &Token) -> bool;
    fn parse(&self, p: &mut Parser<'_>) -> ParseResult<Self::Output>;
}

/// Expression with a specific binding power.
///
/// This is based on [a blog post by Alex Kladov about Pratt
/// parsing][matklad-pratt-parsing].
///
/// [matklad-pratt-parsing]:
///     https://matklad.github.io/2020/04/13/simple-but-powerful-pratt-parsing.html
#[derive(Debug, Default)]
struct Expr(u8);
impl Rule for Expr {
    const NAME: &str = "expression";

    type Output = AstNodeContents;

    fn prefix_matches(&self, token: &Token) -> bool {
        Atom.prefix_matches(token) || matches!(token, Token::Not | Token::Plus | Token::Minus)
    }

    fn parse(&self, p: &mut Parser<'_>) -> ParseResult<Self::Output> {
        todo!()
    }
}

#[derive(Debug, Default)]
struct Atom;
impl Rule for Atom {
    const NAME: &str = "atomic expression";

    type Output = AstNodeContents;

    fn prefix_matches(&self, token: &Token) -> bool {
        matches!(
            token,
            Token::Ident
                | Token::NumberLiteral
                | Token::StringLiteral(_)
                | Token::LBrace
                | Token::LBracket
                | Token::LParen
                | Token::Null
                | Token::True
                | Token::False
                | Token::If
                | Token::Fn
                | Token::Hash
        )
    }

    fn parse(&self, p: &mut Parser<'_>) -> ParseResult<Self::Output> {
        match p.peek() {
            Token::Ident => {
                p.next();
                Ok(AstNodeContents::Ident(p.substr()))
            }
            Token::NumberLiteral => {
                p.next();
                Ok(AstNodeContents::NumberLiteral(
                    p.str().parse().at(p.span())?,
                ))
            }
            Token::StringLiteral(items) => {
                p.next();
                Ok(AstNodeContents::StringLiteral(
                    items
                        .iter()
                        .map(|(item, span)| match item {
                            StringLiteralSegment::Literal => {
                                Ok(StringSegment::Substr(p.source.substr(span.clone())))
                            }
                            StringLiteralSegment::Escape(c) => match *c {
                                'n' => Ok(StringSegment::Char('\n')),
                                'a'..='z' | 'A'..='Z' => {
                                    Err(ParseErrorMsg::BadEscapeChar(*c).at(span.clone()))
                                }
                                other => Ok(StringSegment::Substr(
                                    p.source.substr(span.start + 1..span.end),
                                )),
                            },
                            StringLiteralSegment::Interpolation(tokens) => {
                                Ok(StringSegment::Interpolation(
                                    Parser {
                                        source: p.source,
                                        tokens,
                                        index: None,
                                    }
                                    .parse(Expr(0))?,
                                ))
                            }
                        })
                        .try_collect()?,
                ))
            }
            Token::LBrace => p.parse(Block),
            Token::LBracket => Ok(AstNodeContents::A),
            Token::LParen => Ok(AstNodeContents::A),
            Token::Null => Ok(AstNodeContents::A),
            Token::True => Ok(AstNodeContents::A),
            Token::False => Ok(AstNodeContents::A),
            Token::If => Ok(AstNodeContents::A),
            Token::Fn => Ok(AstNodeContents::A),
            Token::Hash => Ok(AstNodeContents::A),
            _ => p.expected(),
        }
    }
}

fn prefix_binding_power(token: Token) -> Option<((), u8)> {
    match token {
        // Operators
        Token::Plus => Some(((), 102)),
        Token::Minus => Some(((), 102)),
        Token::Bang => Some(((), 102)),
        Token::Tilde => Some(((), 102)),

        // Boolean logic
        Token::Not => Some(((), 16)),

        _ => None,
    }
}

fn infix_binding_power(token: Token) -> Option<(u8, u8)> {
    match token {
        Token::Period => Some((101, 102)),

        // Arithmetic
        Token::DoubleStar => Some((56, 55)), // right-associative
        Token::Star | Token::Slash | Token::Percent => Some((53, 54)),
        Token::Plus | Token::Minus => Some((51, 52)),

        // Bitwise/setwise operators
        Token::RightShift | Token::LeftShift => Some((47, 48)),
        Token::Ampersand => Some((45, 46)),
        Token::Caret => Some((43, 44)),
        Token::Pipe => Some((41, 42)),

        // Type checking
        Token::Is => Some((33, 34)),

        // Null-coalescing
        Token::DoubleQuestionMark => Some((31, 32)),

        // Comparison
        Token::Eq | Token::Neq | Token::Lt | Token::Gt | Token::Lte | Token::Gte => Some((21, 22)),

        // Boolean logic
        Token::And => Some((13, 14)),
        Token::Or => Some((11, 12)),

        // Ranges
        Token::RangeExclusive | Token::RangeInclusive => Some((1, 2)),

        _ => None,
    }
}

fn suffix_binding_power(token: Token) -> Option<(u8, ())> {
    match token {
        Token::LParen => Some((101, ())),
        Token::LBracket => Some((101, ())),

        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq)]
struct ParseError {
    span: Span,
    msg: ParseErrorMsg,
}

#[derive(Error, Debug, Clone, PartialEq)]
enum ParseErrorMsg {
    #[error("{0}")]
    LexError(#[from] LexError),

    #[error("expected {expected:?}; got {got:?}")]
    Expected {
        expected: &'static str,
        got: &'static str,
    },

    #[error("bad number literal: {0}")]
    BadNumber(#[from] ParseFloatError),
    #[error("unknown escape character: {0:?}")]
    BadEscapeChar(char),
}
impl ParseErrorMsg {
    fn expected(expected: impl Into<&'static str>, got: impl Into<&'static str>) -> Self {
        Self::Expected {
            expected: expected.into(),
            got: got.into(),
        }
    }
}
trait ParseErrorAt {
    type Output;
    fn at(self, span: Span) -> Self::Output;
}
impl<T: Into<ParseErrorMsg>> ParseErrorAt for T {
    type Output = ParseError;
    fn at(self, span: Span) -> ParseError {
        ParseError {
            span: span.clone(),
            msg: self.into(),
        }
    }
}
impl<T, E: Into<ParseErrorMsg>> ParseErrorAt for Result<T, E> {
    type Output = Result<T, ParseError>;
    fn at(self, span: Span) -> Self::Output {
        self.map_err(|e| e.at(span))
    }
}

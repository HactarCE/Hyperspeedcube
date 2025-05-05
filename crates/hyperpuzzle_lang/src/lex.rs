use std::{num::ParseFloatError, str::FromStr};

use arcstr::{ArcStr, Substr};
use logos::{Lexer, Logos, Span};
use thiserror::Error;
use unicode_xid::UnicodeXID;

pub fn tokenize(s: &str) -> impl Iterator<Item = (Result<Token, LexError>, Span)> {
    Lexer::new(s).spanned()
}

#[derive(Logos, Debug, Clone)]
#[logos(error = LexError)]
#[logos(extras = LexState)]
#[logos(skip r"[ \t\n\f]+")]
pub enum Token {
    #[regex(r"(_|[^[:punct:]\s])+", priority = 0, callback = |lex| validate_ident(lex.slice()))]
    Ident,
    #[regex(r"-?([0-9]+\.?[0-9]*|\.[0-9]+)", callback = |lex| lex.slice().parse())]
    NumberLiteral(f64),
    #[token("\"", callback = lex_string)]
    StringLiteral(Vec<StringLiteralSegment>),

    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,

    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,

    #[token("(")]
    LParen,
    #[token(")")]
    RParen,

    #[token("true")]
    True,
    #[token("false")]
    False,

    #[token("if")]
    If,
    #[token("else")]
    Else,

    #[token("do")]
    Do,
    #[token("while")]
    While,
    #[token("for")]
    For,
    #[token("in")]
    In,
    #[token("continue")]
    Continue,
    #[token("break")]
    Break,

    #[token("fn")]
    Fn,
    #[token("return")]
    Return,

    #[token("import")]
    Import,
    #[token("export")]
    Export,
    #[token("from")]
    From,
    #[token("as")]
    As,

    #[token("is")]
    Is,

    #[token("and")]
    And,
    #[token("or")]
    Or,
    #[token("not")]
    Not,

    #[token("=")]
    Assign,

    #[token("_")]
    Throwaway,

    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
    #[token("&")]
    Ampersand,
    #[token("|")]
    Pipe,
    #[token("^")]
    Caret,
    #[token("**")]
    DoubleStar,
    #[token("&&")]
    DoubleAmpersand,
    #[token("||")]
    DoublePipe,
    #[token("<<")]
    LeftShift,
    #[token(">>")]
    RightShift,

    #[token("+=")]
    PlusAssign,
    #[token("-=")]
    MinusAssign,
    #[token("*=")]
    StarAssign,
    #[token("/=")]
    SlashAssign,
    #[token("%=")]
    PercentAssign,
    #[token("&=")]
    AmpersandAssign,
    #[token("|=")]
    PipeAssign,
    #[token("^=")]
    CaretAssign,
    #[token("**=")]
    DoubleStarAssign,
    #[token("<<=")]
    LeftShiftAssign,
    #[token(">>=")]
    RightShiftAssign,

    #[token("!")]
    Bang,
    #[token("#")]
    Hash,
    #[token(":")]
    Colon,
    #[token(".")]
    Period,
    #[token("->")]
    Arrow,

    #[token("==")]
    Eq,
    #[token("!=")]
    Neq,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("<=")]
    Lte,
    #[token(">=")]
    Gte,
}

#[derive(Logos, Debug, Clone)]
#[logos(error = LexError)]
#[logos(extras = LexState)]
pub enum StringLiteralSegmentToken {
    #[token("\"")]
    Quote,
    #[regex(r#"[^"$\\]+"#)]
    Content,
    #[regex(r"\\.")]
    Escape,
    #[token("${", lex_interpolation)]
    InterpolationStart(Vec<(Token, Span)>),
    #[token("$")]
    DollarSign,
}

#[derive(Debug, Clone)]
pub enum StringLiteralSegment {
    Literal(String),
    Char(char),
    Interpolation(Vec<(Token, Span)>),
}

fn validate_ident(s: &str) -> Result<(), LexError> {
    let mut chars = s.chars();
    let start_char = chars.next().ok_or(LexError::Internal("empty identifier"))?;
    if !start_char.is_xid_start() {
        return Err(LexError::BadIdentStart(start_char));
    }
    if let Some(bad_char) = chars.find(|c| !c.is_xid_continue()) {
        return Err(LexError::BadIdentContinue(bad_char));
    }
    Ok(())
}

fn lex_string(lex: &mut Lexer<'_, Token>) -> Result<Vec<StringLiteralSegment>, LexError> {
    let mut string_segments_lex = lex.clone().morph::<StringLiteralSegmentToken>();

    let mut segments = vec![];
    let result = loop {
        match string_segments_lex.next() {
            None => break Err(LexError::UnterminatedStringLiteral),
            Some(Err(e)) => break Err(e),
            Some(Ok(StringLiteralSegmentToken::Quote)) => {
                break Ok(segments);
            }
            Some(Ok(StringLiteralSegmentToken::Content)) => segments.push(
                StringLiteralSegment::Literal(string_segments_lex.slice().to_string()),
            ),
            Some(Ok(StringLiteralSegmentToken::Escape)) => {
                match string_segments_lex.slice().chars().nth(1) {
                    Some(c) => segments.push(StringLiteralSegment::Char(c)),
                    None => break Err(LexError::Internal("no escaped char")),
                }
            }
            Some(Ok(StringLiteralSegmentToken::InterpolationStart(tokens))) => {
                segments.push(StringLiteralSegment::Interpolation(tokens))
            }
            Some(Ok(StringLiteralSegmentToken::DollarSign)) => {
                segments.push(StringLiteralSegment::Char('$'))
            }
        }
    };

    *lex = string_segments_lex.morph();
    result
}

fn lex_interpolation(
    lex: &mut Lexer<'_, StringLiteralSegmentToken>,
) -> Result<Vec<(Token, Span)>, LexError> {
    if lex.extras.inside_interpolation {
        dbg!(lex.slice());
        return Err(LexError::NestedInterpolation);
    }

    let mut depth = 1;
    lex.extras.inside_interpolation = true;
    let mut expr_lex = lex.clone().morph::<Token>().spanned();

    let result = std::iter::from_fn(|| expr_lex.next())
        .take_while(|(token, _span)| match token {
            Ok(Token::LBrace) => {
                depth += 1;
                true
            }
            Ok(Token::RBrace) => {
                depth -= 1;
                depth > 0
            }
            _ => true,
        })
        .map(|(token, span)| Ok((token?, span)))
        .collect();

    *lex = (*expr_lex).clone().morph();
    lex.extras.inside_interpolation = false;
    result
}

#[derive(Debug, Default, Clone)]
pub struct LexState {
    /// Whether we are currently inside a string interpolation.
    inside_interpolation: bool,
}

#[derive(Error, Debug, Default, Clone, PartialEq)]
#[non_exhaustive]
pub enum LexError {
    #[default]
    #[error("invalid token")]
    InvalidToken,
    #[error("string literal never ends")]
    UnterminatedStringLiteral,
    #[error("bad numeric literal: {0}")]
    BadNumber(ParseFloatError),
    #[error("bad identifier; {0:?} cannot start an identifier")]
    BadIdentStart(char),
    #[error("bad identifier; {0:?} cannot appear in an identifier")]
    BadIdentContinue(char),

    #[error("string interpolation cannot appear inside another string interpolation")]
    NestedInterpolation,

    #[error("internal error: {0}")]
    Internal(&'static str),
}
impl From<ParseFloatError> for LexError {
    fn from(value: ParseFloatError) -> Self {
        Self::BadNumber(value)
    }
}

fn index_to_line_col_str(s: &str, span: Span) -> String {
    let pre = &s[..span.start];
    let line = pre.chars().filter(|&c| c == '\n').count() + 1;
    let col = match pre.rsplit_once('\n') {
        Some((_, line)) => line.len() + 1,
        None => pre.len() + 1,
    };
    format!("[{line}:{col}]")
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use super::*;

    fn lex_and_serialize(s: &str) -> String {
        serialize_tokens(s, tokenize(s)).into_iter().join("\n")
    }

    fn serialize_tokens(
        s: &str,
        tokens: impl IntoIterator<Item = (Result<Token, LexError>, Span)>,
    ) -> Vec<String> {
        let mut lines = vec![];
        for (token, span) in tokens {
            let span = index_to_line_col_str(s, span);
            match token {
                Ok(Token::StringLiteral(contents)) => {
                    lines.push(format!("{span} StringLiteral"));
                    for segment in contents {
                        match segment {
                            StringLiteralSegment::Interpolation(items) => {
                                for inner_line in serialize_tokens(
                                    s,
                                    items
                                        .into_iter()
                                        .map(|(tok, sp)| (Ok(tok), sp))
                                        .collect_vec(),
                                ) {
                                    lines.push(format!("  {inner_line}"));
                                }
                            }
                            other => lines.push(format!("  {other:?}")),
                        }
                    }
                }
                Ok(other) => lines.push(format!("{span} {other:?}")),
                Err(e) => lines.push(format!("{span} {e}")),
            }
        }
        lines
    }

    fn is_all_whitespace(s: &str) -> bool {
        s.chars().all(|c| c.is_ascii_whitespace())
    }

    #[track_caller]
    fn dedent(s: &str) -> String {
        let indent = s
            .lines()
            .filter(|s| !is_all_whitespace(s))
            .map(|line| line.chars().take_while(|&c| c == ' ').count())
            .min()
            .unwrap_or(0);
        s.trim_end()
            .lines()
            .skip_while(|line| line.is_empty() || is_all_whitespace(line))
            .map(|line| &line[indent..])
            .join("\n")
    }

    #[track_caller]
    fn assert_lexer_output(source: &str, expected_output: &str) {
        assert_eq!(lex_and_serialize(&dedent(source)), dedent(expected_output));
    }

    #[test]
    fn test_lex_string_interpolation() {
        assert_lexer_output(
            r#"
                x = 0
                import * from euclid
                name_it("my ${string} called ${if true { "a" } else { x + 4 }}")
            "#,
            r#"
                [1:1] Ident
                [1:3] Assign
                [1:5] NumberLiteral(0.0)
                [2:1] Import
                [2:8] Star
                [2:10] From
                [2:15] Ident
                [3:1] Ident
                [3:8] LParen
                [3:63] StringLiteral
                  Literal("my ")
                  [3:15] Ident
                  Literal(" called ")
                  [3:32] If
                  [3:35] True
                  [3:40] LBrace
                  [3:44] StringLiteral
                    Literal("a")
                  [3:46] RBrace
                  [3:48] Else
                  [3:53] LBrace
                  [3:55] Ident
                  [3:57] Plus
                  [3:59] NumberLiteral(4.0)
                  [3:61] RBrace
                [3:64] RParen
            "#,
        );
    }

    #[test]
    fn test_lex_nested_string_interpolation() {
        assert_lexer_output(
            r#"
                x = 0
                import * from euclid
                name_it("my string called ${if true { "a${x}" } else { "b" }}")
            "#,
            r#"
                [1:1] Ident
                [1:3] Assign
                [1:5] NumberLiteral(0.0)
                [2:1] Import
                [2:8] Star
                [2:10] From
                [2:15] Ident
                [3:1] Ident
                [3:8] LParen
                [3:41] string interpolation cannot appear inside another string interpolation
                [3:43] Ident
                [3:44] RBrace
                [3:56] StringLiteral
                  Literal(" } else { ")
                [3:57] Ident
                [3:62] StringLiteral
                  Literal(" }}")
                [3:63] RParen
            "#,
        );
    }
}

use logos::{Lexer, Logos};
use strum::Display;
use thiserror::Error;
use unicode_xid::UnicodeXID;

use crate::{Span, Spanned};

pub fn tokenize(s: &str) -> impl Iterator<Item = Spanned<Result<Token, LexError>>> {
    Lexer::new(s).spanned().map(Spanned::from)
}

#[derive(Logos, Display, Debug, Clone, PartialEq, Eq)]
#[logos(error = LexError)]
#[logos(extras = LexState)]
#[logos(skip r"[ \t\n\f]+")]
#[logos(skip r"//[^\n]*")]
pub enum Token {
    #[regex(r"(_|[^[:punct:]\s])+", priority = 0, callback = |lex| validate_ident(lex.slice()))]
    #[strum(to_string = "identifier")]
    Ident,
    #[regex(r"-?([0-9]+\.?[0-9]*|\.[0-9]+)")]
    #[strum(to_string = "numeric literal")]
    NumberLiteral,
    #[token("\"", callback = lex_string)]
    #[strum(to_string = "string literal")]
    StringLiteral(Vec<Spanned<StringLiteralSegment>>),

    #[token("{")]
    #[strum(to_string = "left brace")]
    LBrace,
    #[token("}")]
    #[strum(to_string = "right brace")]
    RBrace,

    #[token("[")]
    #[strum(to_string = "left bracket")]
    LBracket,
    #[token("]")]
    #[strum(to_string = "right bracket")]
    RBracket,

    #[token("(")]
    #[strum(to_string = "left paren")]
    LParen,
    #[token(")")]
    #[strum(to_string = "right paren")]
    RParen,

    #[token("null")]
    #[strum(to_string = "`null`")]
    Null,
    #[token("true")]
    #[strum(to_string = "`true`")]
    True,
    #[token("false")]
    #[strum(to_string = "`false`")]
    False,

    #[token("if")]
    #[strum(to_string = "`if`")]
    If,
    #[token("else")]
    #[strum(to_string = "`else`")]
    Else,

    #[token("do")]
    #[strum(to_string = "`do`")]
    Do,
    #[token("while")]
    #[strum(to_string = "`while`")]
    While,
    #[token("for")]
    #[strum(to_string = "`for`")]
    For,
    #[token("in")]
    #[strum(to_string = "`in`")]
    In,
    #[token("continue")]
    #[strum(to_string = "`continue`")]
    Continue,
    #[token("break")]
    #[strum(to_string = "`break`")]
    Break,
    #[token("return")]
    #[strum(to_string = "`return`")]
    Return,
    #[token("import")]
    #[strum(to_string = "`import`")]
    Import,
    #[token("export")]
    #[strum(to_string = "`export`")]
    Export,
    #[token("fn")]
    #[strum(to_string = "`fn`")]
    Fn,

    #[token("from")]
    #[strum(to_string = "`from`")]
    From,
    #[token("as")]
    #[strum(to_string = "`as`")]
    As,

    #[token("is")]
    #[strum(to_string = "`is`")]
    Is,

    #[token("and")]
    #[strum(to_string = "`and`")]
    And,
    #[token("or")]
    #[strum(to_string = "`or`")]
    Or,
    #[token("not")]
    #[strum(to_string = "`not`")]
    Not,

    #[token(",")]
    #[strum(to_string = "`,`")]
    Comma,

    #[token("+")]
    #[strum(to_string = "`+`")]
    Plus,
    #[token("-")]
    #[strum(to_string = "`-`")]
    Minus,
    #[token("*")]
    #[strum(to_string = "`*`")]
    Star,
    #[token("/")]
    #[strum(to_string = "`/`")]
    Slash,
    #[token("%")]
    #[strum(to_string = "`%`")]
    Percent,
    #[token("&")]
    #[strum(to_string = "`&`")]
    Ampersand,
    #[token("|")]
    #[strum(to_string = "`|`")]
    Pipe,
    #[token("^")]
    #[strum(to_string = "`^`")]
    Caret,
    #[token("~")]
    #[strum(to_string = "`~`")]
    Tilde,
    #[token("**")]
    #[strum(to_string = "`**`")]
    DoubleStar,
    #[token("&&")]
    #[strum(to_string = "`&&`")]
    DoubleAmpersand,
    #[token("||")]
    #[strum(to_string = "`||`")]
    DoublePipe,
    #[token("<<")]
    #[strum(to_string = "`<<`")]
    LeftShift,
    #[token(">>")]
    #[strum(to_string = "`>>`")]
    RightShift,
    #[token("??")]
    #[strum(to_string = "`??`")]
    DoubleQuestionMark,
    #[token("..")]
    #[strum(to_string = "`..`")]
    RangeExclusive,
    #[token("..=")]
    #[strum(to_string = "`..=`")]
    RangeInclusive,

    #[token("=")]
    #[strum(to_string = "`=`")]
    Assign,
    #[token("+=")]
    #[token("-=")]
    #[token("*=")]
    #[token("/=")]
    #[token("%=")]
    #[token("&=")]
    #[token("|=")]
    #[token("^=")]
    #[token("~=")]
    #[token("**=")]
    #[token("<<=")]
    #[token(">>=")]
    #[strum(to_string = "compound assignment operator")]
    CompoundAssign,

    #[token("!")]
    #[strum(to_string = "`!`")]
    Bang,
    #[token("#")]
    #[strum(to_string = "`#`")]
    Hash,
    #[token(":")]
    #[strum(to_string = "`:`")]
    Colon,
    #[token(".")]
    #[strum(to_string = "`.`")]
    Period,
    #[token("->")]
    #[strum(to_string = "`->`")]
    Arrow,

    #[token("==")]
    #[strum(to_string = "`==`")]
    Eql,
    #[token("!=")]
    #[strum(to_string = "`!=`")]
    Neq,
    #[token("<")]
    #[strum(to_string = "`<`")]
    Lt,
    #[token(">")]
    #[strum(to_string = "`>`")]
    Gt,
    #[token("<=")]
    #[strum(to_string = "`<=`")]
    Lte,
    #[token(">=")]
    #[strum(to_string = "`>=`")]
    Gte,

    #[strum(to_string = "end of file")]
    Eof,
}

#[derive(Logos, Debug, Clone)]
#[logos(error = LexError)]
#[logos(extras = LexState)]
enum StringLiteralSegmentToken {
    #[token("\"")]
    Quote,
    #[token("$")]
    #[regex(r#"[^"$\\]+"#)]
    Content,
    #[regex(r"\\.")]
    Escape,
    #[token("${", lex_interpolation)]
    Interpolation(Vec<Spanned<Token>>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StringLiteralSegment {
    Literal,
    Escape(char),
    Interpolation(Vec<Spanned<Token>>),
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

fn lex_string(lex: &mut Lexer<'_, Token>) -> Result<Vec<Spanned<StringLiteralSegment>>, LexError> {
    let mut string_segments_lex = lex.clone().morph::<StringLiteralSegmentToken>().spanned();

    let mut segments = vec![];
    let result = loop {
        match string_segments_lex.next() {
            None => break Err(LexError::UnterminatedStringLiteral),
            Some((Err(e), _)) => break Err(e),
            Some((Ok(token), span)) => {
                let segment = match token {
                    StringLiteralSegmentToken::Quote => break Ok(segments),
                    StringLiteralSegmentToken::Content => StringLiteralSegment::Literal,
                    StringLiteralSegmentToken::Escape => {
                        match string_segments_lex.slice().chars().nth(1) {
                            Some(c) => StringLiteralSegment::Escape(c),
                            None => break Err(LexError::Internal("no escaped char")),
                        }
                    }
                    StringLiteralSegmentToken::Interpolation(tokens) => {
                        StringLiteralSegment::Interpolation(tokens)
                    }
                };
                segments.push(Spanned::new(span, segment))
            }
        }
    };

    *lex = (*string_segments_lex).clone().morph();
    result
}

fn lex_interpolation(
    lex: &mut Lexer<'_, StringLiteralSegmentToken>,
) -> Result<Vec<Spanned<Token>>, LexError> {
    if lex.extras.inside_interpolation {
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
        .map(|(token, span)| Ok(Spanned::new(span, token?)))
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
    #[error("bad identifier; {0:?} cannot start an identifier")]
    BadIdentStart(char),
    #[error("bad identifier; {0:?} cannot appear in an identifier")]
    BadIdentContinue(char),

    #[error("string interpolation cannot appear inside another string interpolation")]
    NestedInterpolation,

    #[error("internal error: {0}")]
    Internal(&'static str),
}

fn index_to_line_col_str(s: &str, span: Span) -> String {
    let pre = &s[..span.start as usize];
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
        tokens: impl IntoIterator<Item = Spanned<Result<Token, LexError>>>,
    ) -> Vec<String> {
        let mut lines = vec![];
        for Spanned { span, inner: token } in tokens {
            let span = index_to_line_col_str(s, span);
            match token {
                Ok(Token::StringLiteral(contents)) => {
                    lines.push(format!("{span} StringLiteral"));
                    for Spanned { span, inner } in contents {
                        let span = index_to_line_col_str(s, span);
                        match inner {
                            StringLiteralSegment::Interpolation(items) => {
                                lines.push(format!("  {span} Interpolation"));
                                for inner_line in serialize_tokens(
                                    s,
                                    items.into_iter().map(|sp| sp.map(Ok)).collect_vec(),
                                ) {
                                    lines.push(format!("    {inner_line}"));
                                }
                            }
                            other => lines.push(format!("  {span} {other:?}")),
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
        let actual = lex_and_serialize(&dedent(source));
        let expected = dedent(expected_output);
        if actual != expected {
            panic!(
                "actual and expected outputs differ.\n\
                 actual:\n\n{actual}\n\n\
                 expected:\n\n{expected}"
            );
        }
    }

    #[test]
    fn test_lex_string_interpolation() {
        assert_lexer_output(
            r#"
                x = 0 // a commment
                import * from euclid//another comment
                name_it("my ${string} called ${if true { "a" } else { x + 4 }}")
                a = "$50 $$ yep"
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
                  [3:10] Literal
                  [3:21] Interpolation
                    [3:15] Ident
                  [3:22] Literal
                  [3:62] Interpolation
                    [3:32] If
                    [3:35] True
                    [3:40] LBrace
                    [3:44] StringLiteral
                      [3:43] Literal
                    [3:46] RBrace
                    [3:48] Else
                    [3:53] LBrace
                    [3:55] Ident
                    [3:57] Plus
                    [3:59] NumberLiteral(4.0)
                    [3:61] RBrace
                [3:64] RParen
                [4:1] Ident
                [4:3] Assign
                [4:5] StringLiteral
                  [4:6] Literal
                  [4:7] Literal
                  [4:10] Literal
                  [4:11] Literal
                  [4:12] Literal
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
                  [3:46] Literal
                [3:57] Ident
                [3:62] StringLiteral
                  [3:59] Literal
                [3:63] RParen
            "#,
        );
    }

    #[test]
    fn test_multiline_string_literal() {
        assert_lexer_output(
            r#"
                s = "multiline
                string
                literal"
            "#,
            r#"
                [1:1] Ident
                [1:3] Assign
                [3:8] StringLiteral
                  [1:6] Literal
            "#,
        );
    }

    #[test]
    fn test_token_to_string() {
        assert_eq!(Token::DoubleQuestionMark.to_string(), "DoubleQuestionMark",);
        assert_eq!(Token::Eof.to_string(), "end of file");
    }
}

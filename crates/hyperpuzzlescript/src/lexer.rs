// use ariadne::{Color, Label, Report, ReportKind, sources};
use chumsky::prelude::*;

use crate::Spanned;

use strum::Display;

pub type LexError<'src> = Rich<'src, char>;

pub fn lexer<'src>()
-> impl Parser<'src, &'src str, Vec<Spanned<Token<'src>>>, extra::Err<LexError<'src>>> {
    recursive(|tokens| {
        let line_comment = just("//").then(any().and_is(just('\n').not()).repeated());
        let block_comment_interior = recursive(|block_comment_interior| {
            choice((
                block_comment_interior.delimited_by(just("/*"), just("*/")),
                just("/").then(any().and_is(just('*').not())).ignored(),
                any().and_is(just('/').not()).ignored(),
            ))
        });
        let block_comment = block_comment_interior.delimited_by(just("/*"), just("*/"));

        let padding = choice((
            one_of(" \t").ignored(),
            line_comment.ignored(),
            block_comment.ignored(),
        ))
        .repeated();

        let ident_or_keyword = chumsky::text::unicode::ident().map(|s: &'src str| match s {
            "null" => Token::Null,
            "true" => Token::True,
            "false" => Token::False,
            "if" => Token::If,
            "else" => Token::Else,
            "do" => Token::Do,
            "while" => Token::While,
            "for" => Token::For,
            "in" => Token::In,
            "continue" => Token::Continue,
            "break" => Token::Break,
            "return" => Token::Return,
            "use" => Token::Use,
            "import" => Token::Import,
            "export" => Token::Export,
            "fn" => Token::Fn,
            "from" => Token::From,
            "as" => Token::As,
            "is" => Token::Is,
            "and" => Token::And,
            "or" => Token::Or,
            "xor" => Token::Xor,
            "not" => Token::Not,

            "inf" => Token::NumberLiteral(s),
            "nan" => Token::NumberLiteral(s),

            _ => Token::Ident,
        });

        let numeric_literal = choice((
            text::int(10)
                .then(just('.').then(text::digits(10)).or_not())
                .to_slice(),
            just('.').then(text::digits(10).at_least(1)).to_slice(),
        ))
        .labelled("number literal");

        let string_interpolation = tokens.clone().delimited_by(just("${"), just('}'));
        let string_escape = just("\\").ignore_then(any()).ignore_then(choice((
            choice((just('\\'), just('$'), just('"'), just('\''))),
            just('n').to('\n'),
        )));
        let string_literal = choice((
            none_of("\"\\$")
                .repeated()
                .at_least(1)
                .to(StringSegmentToken::Literal),
            string_escape.map(StringSegmentToken::Escape),
            string_interpolation.map(StringSegmentToken::Interpolation),
        ))
        .map_with(|tok, extra| (tok, extra.span()))
        .repeated()
        .collect()
        .delimited_by(just('"'), just('"'))
        .map(Token::StringLiteral);

        choice((
            ident_or_keyword,
            numeric_literal.map(Token::NumberLiteral),
            string_literal,
            // For string interpolation to work correctly, we need to parse
            // bracket pairs during lexing.
            tokens
                .clone()
                .delimited_by(just('('), just(')'))
                .map(Token::Parens),
            tokens
                .clone()
                .delimited_by(just('['), just(']'))
                .map(Token::Brackets),
            tokens
                .clone()
                .delimited_by(just('{'), just('}'))
                .map(Token::Braces),
            choice([
                just("**="),
                just("<<="),
                just(">>="),
                just("??="),
                just("+="),
                just("-="),
                just("*="),
                just("/="),
                just("%="),
                just("&="),
                just("|="),
                just("^="),
            ])
            .to(Token::CompoundAssign),
            choice([
                just("\n").to(Token::Newline),
                just("**").to(Token::DoubleStar),
                just("&&").to(Token::DoubleAmpersand),
                just("||").to(Token::DoublePipe),
                just("<<").to(Token::LeftShift),
                just(">>").to(Token::RightShift),
                just(".|").to(Token::LeftContract),
                just("|.").to(Token::RightContract),
                just("??").to(Token::DoubleQuestionMark),
                just("..=").to(Token::RangeInclusive),
                just("..").to(Token::RangeExclusive),
                just("->").to(Token::Arrow),
                just("==").to(Token::Eql),
                just("!=").to(Token::Neq),
                just("<=").to(Token::Lte),
                just(">=").to(Token::Gte),
                just("<").to(Token::Lt),
                just(">").to(Token::Gt),
                just("=").to(Token::Assign),
                just(",").to(Token::Comma),
                just("+").to(Token::Plus),
                just("-").to(Token::Minus),
                just("*").to(Token::Star),
                just("/").to(Token::Slash),
                just("%").to(Token::Percent),
                just("&").to(Token::Ampersand),
                just("|").to(Token::Pipe),
                just("^").to(Token::Caret),
                just("~").to(Token::Tilde),
                just("!").to(Token::Bang),
                just("#").to(Token::Hash),
                just(":").to(Token::Colon),
                just(".").to(Token::Period),
            ]),
        ))
        .map_with(|tok, extra| (tok, extra.span()))
        .padded_by(padding)
        .repeated()
        .collect()
    })
}

#[derive(Display, Debug, Clone, PartialEq, Eq)]
pub enum Token<'src> {
    #[strum(to_string = "identifier")]
    Ident,
    #[strum(to_string = "number literal")]
    NumberLiteral(&'src str),
    #[strum(to_string = "string literal")]
    StringLiteral(Vec<Spanned<StringSegmentToken<'src>>>),

    #[strum(to_string = "`(...)`")]
    Parens(Vec<Spanned<Token<'src>>>),
    #[strum(to_string = "`[...]`")]
    Brackets(Vec<Spanned<Token<'src>>>),
    #[strum(to_string = "`{{...}}`")] // TODO: confirm that this renders as one symbol
    Braces(Vec<Spanned<Token<'src>>>),

    #[strum(to_string = "`null`")]
    Null,
    #[strum(to_string = "`true`")]
    True,
    #[strum(to_string = "`false`")]
    False,

    #[strum(to_string = "`if`")]
    If,
    #[strum(to_string = "`else`")]
    Else,

    #[strum(to_string = "`do`")]
    Do,
    #[strum(to_string = "`while`")]
    While,
    #[strum(to_string = "`for`")]
    For,
    #[strum(to_string = "`in`")]
    In,
    #[strum(to_string = "`continue`")]
    Continue,
    #[strum(to_string = "`break`")]
    Break,
    #[strum(to_string = "`return`")]
    Return,
    #[strum(to_string = "`use`")]
    Use,
    #[strum(to_string = "`import`")]
    Import,
    #[strum(to_string = "`export`")]
    Export,
    #[strum(to_string = "`fn`")]
    Fn,

    #[strum(to_string = "`from`")]
    From,
    #[strum(to_string = "`as`")]
    As,

    #[strum(to_string = "`is`")]
    Is,

    #[strum(to_string = "`and`")]
    And,
    #[strum(to_string = "`or`")]
    Or,
    #[strum(to_string = "`xor`")]
    Xor,
    #[strum(to_string = "`not`")]
    Not,

    #[strum(to_string = "newline")]
    Newline,

    #[strum(to_string = "`,`")]
    Comma,

    #[strum(to_string = "`+`")]
    Plus,
    #[strum(to_string = "`-`")]
    Minus,
    #[strum(to_string = "`*`")]
    Star,
    #[strum(to_string = "`/`")]
    Slash,
    #[strum(to_string = "`%`")]
    Percent,
    #[strum(to_string = "`&`")]
    Ampersand,
    #[strum(to_string = "`|`")]
    Pipe,
    #[strum(to_string = "`^`")]
    Caret,
    #[strum(to_string = "`~`")]
    Tilde,
    #[strum(to_string = "`**`")]
    DoubleStar,
    #[strum(to_string = "`&&`")]
    DoubleAmpersand,
    #[strum(to_string = "`||`")]
    DoublePipe,
    #[strum(to_string = "`<<`")]
    LeftShift,
    #[strum(to_string = "`>>`")]
    RightShift,
    #[strum(to_string = "`.|`")]
    LeftContract,
    #[strum(to_string = "`|.`")]
    RightContract,
    #[strum(to_string = "`??`")]
    DoubleQuestionMark,
    #[strum(to_string = "`..=`")]
    RangeInclusive,
    #[strum(to_string = "`..`")]
    RangeExclusive,

    #[strum(to_string = "`!`")]
    Bang,
    #[strum(to_string = "`#`")]
    Hash,
    #[strum(to_string = "`:`")]
    Colon,
    #[strum(to_string = "`.`")]
    Period,
    #[strum(to_string = "`->`")]
    Arrow,

    #[strum(to_string = "`==`")]
    Eql,
    #[strum(to_string = "`!=`")]
    Neq,
    #[strum(to_string = "`<`")]
    Lt,
    #[strum(to_string = "`>`")]
    Gt,
    #[strum(to_string = "`<=`")]
    Lte,
    #[strum(to_string = "`>=`")]
    Gte,

    #[strum(to_string = "`=`")]
    Assign,
    #[strum(to_string = "compound assignment operator")]
    CompoundAssign,

    #[strum(to_string = "end of file")]
    Eof,
}

// #[derive(Debug, Clone)]
// enum StringLiteralSegmentToken {
//     Quote,
//     Content,
//     Escape(char),
//     Interpolation(Vec<Spanned<Token>>),
// }

#[derive(Display, Debug, Clone, PartialEq, Eq)]
pub enum StringSegmentToken<'src> {
    #[strum(to_string = "literal text")]
    Literal,
    #[strum(to_string = "escape sequence using backslash")]
    Escape(char),
    #[strum(to_string = "${{...}}")] // TODO: make sure this renders correctly
    Interpolation(Vec<Spanned<Token<'src>>>),
}

// fn validate_ident(s: &str) -> Result<(), LexError> {
//     let mut chars = s.chars();
//     let start_char = chars.next().ok_or(LexError::Internal("empty identifier"))?;
//     if !start_char.is_xid_start() {
//         return Err(LexError::BadIdentStart(start_char));
//     }
//     if let Some(bad_char) = chars.find(|c| !c.is_xid_continue()) {
//         return Err(LexError::BadIdentContinue(bad_char));
//     }
//     Ok(())
// }

// fn lex_string(lex: &mut Lexer<'_, Token>) -> Result<Vec<Spanned<StringSegmentToken>>, LexError> {
//     let mut string_segments_lex = lex.clone().morph::<StringLiteralSegmentToken>().spanned();

//     let mut segments = vec![];
//     let result = loop {
//         match string_segments_lex.next() {
//             None => break Err(LexError::UnterminatedStringLiteral),
//             Some((Err(e), _)) => break Err(e),
//             Some((Ok(token), span)) => {
//                 let segment = match token {
//                     StringLiteralSegmentToken::Quote => break Ok(segments),
//                     StringLiteralSegmentToken::Content => StringSegmentToken::Literal,
//                     StringLiteralSegmentToken::Escape => {
//                         match string_segments_lex.slice().chars().nth(1) {
//                             Some(c) => StringSegmentToken::Escape(c),
//                             None => break Err(LexError::Internal("no escaped char")),
//                         }
//                     }
//                     StringLiteralSegmentToken::Interpolation(tokens) => {
//                         StringSegmentToken::Interpolation(tokens)
//                     }
//                 };
//                 segments.push(Spanned::new(span, segment))
//             }
//         }
//     };

//     *lex = (*string_segments_lex).clone().morph();
//     result
// }

// fn lex_interpolation(
//     lex: &mut Lexer<'_, StringLiteralSegmentToken>,
// ) -> Result<Vec<Spanned<Token>>, LexError> {
//     if lex.extras.inside_interpolation {
//         return Err(LexError::NestedInterpolation);
//     }

//     let mut depth = 1;
//     lex.extras.inside_interpolation = true;
//     let mut expr_lex = lex.clone().morph::<Token>().spanned();

//     let result = std::iter::from_fn(|| expr_lex.next())
//         .take_while(|(token, _span)| match token {
//             Ok(Token::LBrace) => {
//                 depth += 1;
//                 true
//             }
//             Ok(Token::RBrace) => {
//                 depth -= 1;
//                 depth > 0
//             }
//             _ => true,
//         })
//         .map(|(token, span)| Ok(Spanned::new(span, token?)))
//         .collect();

//     *lex = (*expr_lex).clone().morph();
//     lex.extras.inside_interpolation = false;
//     result
// }

// #[derive(Debug, Default, Clone)]
// pub struct LexState {
//     /// Whether we are currently inside a string interpolation.
//     inside_interpolation: bool,
// }

// #[derive(Error, Debug, Default, Clone, PartialEq)]
// #[non_exhaustive]
// pub enum LexError {
//     #[default]
//     #[error("invalid token")]
//     InvalidToken,
//     #[error("string literal never ends")]
//     UnterminatedStringLiteral,
//     #[error("bad identifier; {0:?} cannot start an identifier")]
//     BadIdentStart(char),
//     #[error("bad identifier; {0:?} cannot appear in an identifier")]
//     BadIdentContinue(char),

//     #[error("string interpolation cannot appear inside another string interpolation")]
//     NestedInterpolation,

//     #[error("internal error: {0}")]
//     Internal(&'static str),
// }

// fn index_to_line_col_str(s: &str, span: Span) -> String {
//     let pre = &s[..span.start as usize];
//     let line = pre.chars().filter(|&c| c == '\n').count() + 1;
//     let col = match pre.rsplit_once('\n') {
//         Some((_, line)) => line.len() + 1,
//         None => pre.len() + 1,
//     };
//     format!("[{line}:{col}]")
// }

// #[cfg(test)]
// mod tests {
//     use itertools::Itertools;

//     use super::*;

//     fn lex_and_serialize(s: &str) -> String {
//         serialize_tokens(s, tokenize(s)).into_iter().join("\n")
//     }

//     fn serialize_tokens(
//         s: &str,
//         tokens: impl IntoIterator<Item = Spanned<Result<Token, LexError>>>,
//     ) -> Vec<String> {
//         let mut lines = vec![];
//         for Spanned { span, inner: token } in tokens {
//             let span = index_to_line_col_str(s, span);
//             match token {
//                 Ok(Token::StringLiteral(contents)) => {
//                     lines.push(format!("{span} StringLiteral"));
//                     for Spanned { span, inner } in contents {
//                         let span = index_to_line_col_str(s, span);
//                         match inner {
//                             StringSegmentToken::Interpolation(items) => {
//                                 lines.push(format!("  {span} Interpolation"));
//                                 for inner_line in serialize_tokens(
//                                     s,
//                                     items.into_iter().map(|sp| sp.map(Ok)).collect_vec(),
//                                 ) {
//                                     lines.push(format!("    {inner_line}"));
//                                 }
//                             }
//                             other => lines.push(format!("  {span} {other:?}")),
//                         }
//                     }
//                 }
//                 Ok(other) => lines.push(format!("{span} {other:?}")),
//                 Err(e) => lines.push(format!("{span} {e}")),
//             }
//         }
//         lines
//     }

//     fn is_all_whitespace(s: &str) -> bool {
//         s.chars().all(|c| c.is_ascii_whitespace())
//     }

//     #[track_caller]
//     fn dedent(s: &str) -> String {
//         let indent = s
//             .lines()
//             .filter(|s| !is_all_whitespace(s))
//             .map(|line| line.chars().take_while(|&c| c == ' ').count())
//             .min()
//             .unwrap_or(0);
//         s.trim_end()
//             .lines()
//             .skip_while(|line| line.is_empty() || is_all_whitespace(line))
//             .map(|line| &line[indent..])
//             .join("\n")
//     }

//     #[track_caller]
//     fn assert_lexer_output(source: &str, expected_output: &str) {
//         let actual = lex_and_serialize(&dedent(source));
//         let expected = dedent(expected_output);
//         if actual != expected {
//             panic!(
//                 "actual and expected outputs differ.\n\
//                  actual:\n\n{actual}\n\n\
//                  expected:\n\n{expected}"
//             );
//         }
//     }

//     #[test]
//     fn test_lex_string_interpolation() {
//         assert_lexer_output(
//             r#"
//                 x = 0 // a commment
//                 import * from euclid//another comment
//                 name_it("my ${string} called ${if true { "a" } else { x + 4 }}")
//                 a = "$50 $$ yep"
//             "#,
//             r#"
//                 [1:1] Ident
//                 [1:3] Assign
//                 [1:5] NumberLiteral(0.0)
//                 [2:1] Import
//                 [2:8] Star
//                 [2:10] From
//                 [2:15] Ident
//                 [3:1] Ident
//                 [3:8] LParen
//                 [3:63] StringLiteral
//                   [3:10] Literal
//                   [3:21] Interpolation
//                     [3:15] Ident
//                   [3:22] Literal
//                   [3:62] Interpolation
//                     [3:32] If
//                     [3:35] True
//                     [3:40] LBrace
//                     [3:44] StringLiteral
//                       [3:43] Literal
//                     [3:46] RBrace
//                     [3:48] Else
//                     [3:53] LBrace
//                     [3:55] Ident
//                     [3:57] Plus
//                     [3:59] NumberLiteral(4.0)
//                     [3:61] RBrace
//                 [3:64] RParen
//                 [4:1] Ident
//                 [4:3] Assign
//                 [4:5] StringLiteral
//                   [4:6] Literal
//                   [4:7] Literal
//                   [4:10] Literal
//                   [4:11] Literal
//                   [4:12] Literal
//             "#,
//         );
//     }

//     #[test]
//     fn test_lex_nested_string_interpolation() {
//         assert_lexer_output(
//             r#"
//                 x = 0
//                 import * from euclid
//                 name_it("my string called ${if true { "a${x}" } else { "b" }}")
//             "#,
//             r#"
//                 [1:1] Ident
//                 [1:3] Assign
//                 [1:5] NumberLiteral(0.0)
//                 [2:1] Import
//                 [2:8] Star
//                 [2:10] From
//                 [2:15] Ident
//                 [3:1] Ident
//                 [3:8] LParen
//                 [3:41] string interpolation cannot appear inside another string interpolation
//                 [3:43] Ident
//                 [3:44] RBrace
//                 [3:56] StringLiteral
//                   [3:46] Literal
//                 [3:57] Ident
//                 [3:62] StringLiteral
//                   [3:59] Literal
//                 [3:63] RParen
//             "#,
//         );
//     }

//     #[test]
//     fn test_multiline_string_literal() {
//         assert_lexer_output(
//             r#"
//                 s = "multiline
//                 string
//                 literal"
//             "#,
//             r#"
//                 [1:1] Ident
//                 [1:3] Assign
//                 [3:8] StringLiteral
//                   [1:6] Literal
//             "#,
//         );
//     }

//     #[test]
//     fn test_token_to_string() {
//         assert_eq!(Token::DoubleQuestionMark.to_string(), "DoubleQuestionMark",);
//         assert_eq!(Token::Eof.to_string(), "end of file");
//     }
// }

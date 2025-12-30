use std::fmt;

use chumsky::prelude::*;

use crate::{FileId, Span, Spanned};

pub(super) type LexError<'src> = Rich<'src, char, SimpleSpan>;
pub(super) type LexState = extra::SimpleState<FileId>;
pub(super) type LexExtra<'src> = extra::Full<LexError<'src>, LexState, ()>;
type LexExtraInternal<'src> = extra::Full<LexError<'src>, LexState, LexCtx>;

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
struct LexCtx {
    ignore_newlines: bool,
}
impl LexCtx {
    const IGNORE_NEWLINES: Self = Self {
        ignore_newlines: true,
    };
}

/// Adds a label to an error at `span` saying "inside here".
// TODO: figure out why this never gets called
fn inside_here<'src>(
    mut e: LexError<'src>,
    span: SimpleSpan,
    _state: &mut LexState,
) -> LexError<'src> {
    chumsky::label::LabelError::<&str, _>::in_context(&mut e, "inside here".to_string(), span);
    e
}

/// Returns a [`Span`] from chumsky "extra" data.
fn span_from_extra<'src>(
    extra: &mut chumsky::input::MapExtra<'src, '_, &'src str, LexExtraInternal<'src>>,
) -> Span {
    super::span_with_file(**extra.state(), extra.span())
}

fn ident_or_keyword<'src, E: extra::ParserExtra<'src, &'src str, Error = LexError<'src>>>()
-> impl Clone + Parser<'src, &'src str, Token, E> {
    chumsky::text::unicode::ident().map(|s: &'src str| match s {
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
        "with" => Token::With,
        "from" => Token::From,
        "as" => Token::As,
        "is" => Token::Is,
        "and" => Token::And,
        "or" => Token::Or,
        "xor" => Token::Xor,
        "not" => Token::Not,

        _ => Token::Ident,
    })
}

pub fn is_valid_ident(s: &str) -> bool {
    ident_or_keyword::<extra::Err<_>>().parse(s).output() == Some(&Token::Ident)
}

pub fn lexer<'src>() -> impl Parser<'src, &'src str, Vec<Spanned<Token>>, LexExtra<'src>> {
    recursive(|tokens| {
        let line_comment = just("//").then(any().and_is(just('\n').not()).repeated());
        // TODO: improve error message on block comment mismatch
        let block_comment = recursive(|block_comment| {
            choice((
                block_comment,
                just("/").then(any().and_is(just('*').not())).ignored(),
                just("*").then(any().and_is(just('/').not())).ignored(),
                any().and_is(one_of("/*").not()).ignored(),
            ))
            .repeated()
            .delimited_by(just("/*"), just("*/"))
        });

        let padding = choice((
            one_of(" \t\r").ignored(),
            line_comment.ignored(),
            block_comment.ignored(),
            just('\n')
                .ignored()
                .contextual()
                .configure(|_, ctx: &LexCtx| ctx.ignore_newlines),
        ))
        .repeated()
        .boxed();

        let ident_or_keyword = ident_or_keyword();

        let special_ident = just('#')
            .then(ident_or_keyword.clone())
            .to(Token::SpecialIdent);

        let numeric_literal = choice((
            text::int(10)
                .then(just('.').then(text::digits(10)).or_not())
                .to_slice(),
            just('.').then(text::digits(10).at_least(1)).to_slice(),
        ))
        .to(Token::NumberLiteral)
        .labelled("number literal");

        let string_interpolation = tokens
            .clone()
            .with_ctx(LexCtx::IGNORE_NEWLINES)
            .delimited_by(just("${"), just('}'));
        let string_escape = just("\\").ignore_then(any().try_map_with(|c, e| match c {
            't' => Ok('\t'),
            'r' => Ok('\r'),
            'n' => Ok('\n'),
            _ if c.is_ascii_punctuation() => Ok(c),
            _ => Err(Rich::custom(
                e.span(),
                format!("unknown escape character: {c:?}"),
            )),
        }));
        let string_literal = choice((
            none_of(super::CHARS_THAT_MUST_BE_ESCAPED_IN_STRING_LITERALS)
                .repeated()
                .at_least(1)
                .to(StringSegmentToken::Literal),
            string_escape.map(StringSegmentToken::Escape),
            string_interpolation.map(StringSegmentToken::Interpolation),
        ))
        .map_with(|tok, e| (tok, span_from_extra(e)))
        .repeated()
        .collect()
        .delimited_by(just('"'), just('"'))
        .map(Token::StringLiteral);

        let map_literal = tokens
            .clone()
            .map_err_with_state(inside_here)
            .with_ctx(LexCtx::IGNORE_NEWLINES)
            .delimited_by(just("#{"), just('}'))
            .map(Token::MapLiteral);

        let file_path = just('@')
            .then(just('^').repeated())
            .then(
                any()
                    .try_map(|c: char, span| {
                        if c.is_alphanumeric() || c == '_' {
                            Ok(c)
                        } else {
                            Err(chumsky::label::LabelError::<&str, _>::expected_found(
                                [chumsky::text::TextExpected::<&str>::AnyIdentifier],
                                Some(chumsky::util::MaybeRef::Val(c)),
                                span,
                            ))
                        }
                    })
                    .repeated()
                    .separated_by(just('/'))
                    .allow_leading(),
            )
            .to(Token::FilePath);

        choice((
            ident_or_keyword,
            special_ident,
            numeric_literal,
            string_literal,
            map_literal,
            file_path,
            // For string interpolation to work correctly, we need to parse
            // bracket pairs during lexing.
            tokens
                .clone()
                .with_ctx(LexCtx::IGNORE_NEWLINES)
                .delimited_by(just('('), just(')'))
                .map(Token::Parens),
            tokens
                .clone()
                .map_err_with_state(inside_here)
                .with_ctx(LexCtx::IGNORE_NEWLINES)
                .delimited_by(just('['), just(']'))
                .map(Token::Brackets),
            tokens
                .clone()
                .map_err_with_state(inside_here)
                .with_ctx(LexCtx::default())
                .delimited_by(just('{'), just('}'))
                .map(Token::Braces),
            choice([
                just("++="),
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
                just("++").to(Token::DoublePlus),
                just("**").to(Token::DoubleStar),
                just("&&").to(Token::DoubleAmpersand),
                just("||").to(Token::DoublePipe),
                just("<<").to(Token::LeftShift),
                just(">>").to(Token::RightShift),
                just(".|").to(Token::LeftContract),
                just("|.").to(Token::RightContract),
                just("\\/").to(Token::Wedge),
                just("/\\").to(Token::Antiwedge),
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
                just("?").to(Token::QuestionMark),
                just("#").to(Token::Hash),
                just(":").to(Token::Colon),
                just(".").to(Token::Period),
                just("√").to(Token::Sqrt),
                just("°").to(Token::Degrees),
                just("∞").to(Token::Ident),
            ]),
        ))
        .labelled("token")
        .map_with(|tok, e| (tok, span_from_extra(e)))
        .padded_by(padding.clone())
        .repeated()
        .collect()
        .padded_by(padding)
    })
    .map_err_with_state(inside_here)
    .with_ctx(LexCtx::default())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Ident,
    SpecialIdent,
    NumberLiteral,
    StringLiteral(Vec<Spanned<StringSegmentToken>>),
    MapLiteral(Vec<Spanned<Token>>),
    FilePath,

    Parens(Vec<Spanned<Token>>),
    Brackets(Vec<Spanned<Token>>),
    Braces(Vec<Spanned<Token>>),

    Null,
    True,
    False,

    If,
    Else,

    Do,
    While,
    For,
    In,
    Continue,
    Break,
    Return,
    Use,
    Import,
    Export,
    Fn,
    With,

    From,
    As,

    Is,

    And,
    Or,
    Xor,
    Not,

    Newline,

    Comma,

    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Ampersand,
    Pipe,
    Caret,
    Tilde,
    DoublePlus,
    DoubleStar,
    DoubleAmpersand,
    DoublePipe,
    LeftShift,
    RightShift,
    LeftContract,
    RightContract,
    Wedge,
    Antiwedge,
    DoubleQuestionMark,
    RangeInclusive,
    RangeExclusive,

    Bang,
    QuestionMark,
    Hash,
    Colon,
    Period,
    Arrow,

    Eql,
    Neq,
    Lt,
    Gt,
    Lte,
    Gte,

    Assign,
    CompoundAssign,

    Sqrt,
    Degrees,
}
impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Ident => "<identifier>",
            Self::SpecialIdent => "<special identifier>",
            Self::NumberLiteral => "<number literal>",
            Self::StringLiteral(_) => "<string literal>",
            Self::MapLiteral(_) => "#{...}",
            Self::FilePath => "file path",
            Self::Parens(_) => "(...)",
            Self::Brackets(_) => "[...]",
            Self::Braces(_) => "{...}",
            Self::Null => "null",
            Self::True => "true",
            Self::False => "false",
            Self::If => "if",
            Self::Else => "else",
            Self::Do => "do",
            Self::While => "while",
            Self::For => "for",
            Self::In => "in",
            Self::Continue => "continue",
            Self::Break => "break",
            Self::Return => "return",
            Self::Use => "use",
            Self::Import => "import",
            Self::Export => "export",
            Self::Fn => "fn",
            Self::With => "with",
            Self::From => "from",
            Self::As => "as",
            Self::Is => "is",
            Self::And => "and",
            Self::Or => "or",
            Self::Xor => "xor",
            Self::Not => "not",
            Self::Newline => "newline",
            Self::Comma => ",",
            Self::Plus => "+",
            Self::Minus => "-",
            Self::Star => "*",
            Self::Slash => "/",
            Self::Percent => "%",
            Self::Ampersand => "&",
            Self::Pipe => "|",
            Self::Caret => "^",
            Self::Tilde => "~",
            Self::DoublePlus => "++",
            Self::DoubleStar => "**",
            Self::DoubleAmpersand => "&&",
            Self::DoublePipe => "||",
            Self::LeftShift => "<<",
            Self::RightShift => ">>",
            Self::LeftContract => ".|",
            Self::RightContract => "|.",
            Self::Wedge => "\\/",
            Self::Antiwedge => "/\\",
            Self::DoubleQuestionMark => "??",
            Self::RangeInclusive => "..=",
            Self::RangeExclusive => "..",
            Self::Bang => "!",
            Self::QuestionMark => "?",
            Self::Hash => "#",
            Self::Colon => ":",
            Self::Period => ".",
            Self::Arrow => "->",
            Self::Eql => "==",
            Self::Neq => "!=",
            Self::Lt => "<",
            Self::Gt => ">",
            Self::Lte => "<=",
            Self::Gte => ">=",
            Self::CompoundAssign => "<compound assignment operator>",
            Self::Assign => "=",
            Self::Sqrt => "√",
            Self::Degrees => "°",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StringSegmentToken {
    Literal,
    Escape(char),
    Interpolation(Vec<Spanned<Token>>),
}
impl fmt::Display for StringSegmentToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            StringSegmentToken::Literal => "literal text",
            StringSegmentToken::Escape(_) => "escape sequence using `\\`",
            StringSegmentToken::Interpolation(_) => "${...}",
        };
        write!(f, "{s}")
    }
}

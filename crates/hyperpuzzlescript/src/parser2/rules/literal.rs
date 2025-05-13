use super::*;

/// Matches an identifier.
#[derive(Debug, Copy, Clone)]
pub struct Ident;
impl_display!(for Ident, "identifier");
impl SyntaxRule for Ident {
    type Output = Substr;

    fn prefix_matches(&self, mut p: Parser<'_>) -> bool {
        *p.next() == Token::Ident
    }

    fn consume_match(self, p: &mut Parser<'_>) -> ParseResult<Self::Output> {
        p.parse(Token::Ident.with_span())
            .map(|span| p.source.substr(span.range()))
    }
}

/// Matches a string literal.
#[derive(Debug, Copy, Clone)]
pub struct StringLiteral;
impl_display!(for StringLiteral, "string literal");
impl SyntaxRule for StringLiteral {
    type Output = ast::NodeContents;

    fn prefix_matches(&self, mut p: Parser<'_>) -> bool {
        matches!(p.next(), Token::StringLiteral(_))
    }
    fn consume_match(self, p: &mut Parser<'_>) -> ParseResult<Self::Output> {
        let Token::StringLiteral(segments) = p.next() else {
            return p.expected(self);
        };
        let segments = segments.iter().map(|Spanned { span, inner }| match inner {
            StringLiteralSegment::Literal => {
                Ok(ast::StringSegment::Substr(p.source.substr(span.range())))
            }
            StringLiteralSegment::Escape(c) => match *c {
                'n' => Ok(ast::StringSegment::Char('\n')),
                'a'..='z' | 'A'..='Z' => Err(ParseErrorMsg::BadEscapeChar(*c).at(*span)),
                _ => {
                    let mut span = *span;
                    span.start += 1; // skip backslash
                    Ok(ast::StringSegment::Substr(p.source.substr(span.range())))
                }
            },
            StringLiteralSegment::Interpolation(tokens) => Ok(ast::StringSegment::Interpolation(
                Parser {
                    source: p.source,
                    tokens,
                    cursor: None,
                }
                .parse(expr::Expr)?,
            )),
        });
        Ok(ast::NodeContents::StringLiteral(segments.try_collect()?))
    }
}

/// Matches a list literal.
#[derive(Debug, Copy, Clone)]
pub struct ListLiteral;
impl_display!(for ListLiteral, "list literal");
impl SyntaxRule for ListLiteral {
    type Output = ast::NodeContents;

    fn prefix_matches(&self, mut p: Parser<'_>) -> bool {
        *p.next() == Token::LBracket
    }

    fn consume_match(self, p: &mut Parser<'_>) -> ParseResult<Self::Output> {
        p.parse(combinators::List {
            inner: expr::Expr,
            sep: Token::Comma,
            start: Token::LBracket,
            end: Token::RBracket,
            allow_trailing_sep: true,
            allow_empty: true,
        })
        .map(ast::NodeContents::ListLiteral)
    }
}

/// Matches a map literal.
#[derive(Debug, Copy, Clone)]
pub struct MapLiteral;
impl_display!(for MapLiteral, "map literal");
impl SyntaxRule for MapLiteral {
    type Output = ast::NodeContents;

    fn prefix_matches(&self, mut p: Parser<'_>) -> bool {
        *p.next() == Token::Hash
    }

    fn consume_match(self, p: &mut Parser<'_>) -> ParseResult<Self::Output> {
        p.parse(Token::Hash)?;
        p.parse(combinators::List {
            inner: MapEntry,
            sep: Token::Comma,
            start: Token::LBrace,
            end: Token::RBrace,
            allow_trailing_sep: true,
            allow_empty: true,
        })
        .map(ast::NodeContents::MapLiteral)
    }
}

/// Matches a map entry.
#[derive(Debug, Copy, Clone)]
pub struct MapEntry;
impl_display!(for MapEntry, "map entry");
impl SyntaxRule for MapEntry {
    type Output = (ast::Node, ast::Node);

    fn prefix_matches(&self, _p: Parser<'_>) -> bool {
        true
    }

    fn consume_match(self, p: &mut Parser<'_>) -> ParseResult<Self::Output> {
        let key = parse_one_of!(
            p,
            [
                Ident.map(ast::NodeContents::Ident).with_span(),
                StringLiteral.with_span()
            ]
        )?;
        p.parse(Token::Colon);
        let value = p.parse(expr::Expr)?;
        Ok((key, value))
    }
}

/// Matches a closure.
#[derive(Debug, Copy, Clone)]
pub struct AnonymousFn;
impl_display!(for AnonymousFn, "anonymous function");
impl SyntaxRule for AnonymousFn {
    type Output = ast::NodeContents;

    fn prefix_matches(&self, mut p: Parser<'_>) -> bool {
        *p.next() == Token::Fn
    }

    fn consume_match(self, p: &mut Parser<'_>) -> ParseResult<Self::Output> {
        p.parse(Token::Fn)?;
        p.parse(FnContents).map(ast::NodeContents::Fn)
    }
}

/// Matches the contents of a function (everything after `fn` and an optional
/// name).
#[derive(Debug, Copy, Clone)]
pub struct FnContents;
impl_display!(for FnContents, "function arguments and body");
impl SyntaxRule for FnContents {
    type Output = ast::FnContents;

    fn prefix_matches(&self, _p: Parser<'_>) -> bool {
        true // always required
    }

    fn consume_match(self, p: &mut Parser<'_>) -> ParseResult<Self::Output> {
        let params = p.parse(combinators::List {
            inner: TypedIdent,
            sep: Token::Comma,
            start: Token::LParen,
            end: Token::RParen,
            allow_trailing_sep: true,
            allow_empty: true,
        })?;
        let return_type = match p.try_parse(Token::Arrow).is_some() {
            true => Some(Box::new(p.parse(expr::Expr)?)),
            false => None,
        };
        let body = p.parse(stmt::Block)?;
        Ok(ast::FnContents {
            params,
            return_type,
            body,
        })
    }
}

/// Matches an identifier with an optional type annotation.
#[derive(Debug, Copy, Clone)]
struct TypedIdent;
impl_display!(for TypedIdent, "identifier with optional type annotation");
impl SyntaxRule for TypedIdent {
    type Output = ast::FnParam;

    fn prefix_matches(&self, mut p: Parser<'_>) -> bool {
        *p.next() == Token::Ident
    }

    fn consume_match(self, p: &mut Parser<'_>) -> ParseResult<Self::Output> {
        let name = p.parse(Ident)?;
        let ty = match p.try_parse(Token::Comma).is_some() {
            true => Some(Box::new(p.parse(expr::Expr)?)),
            false => None,
        };
        Ok(ast::FnParam { name, ty })
    }
}

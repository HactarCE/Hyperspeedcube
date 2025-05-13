use super::*;

/// Rule that adds a span.
#[derive(Debug, Default, Copy, Clone)]
pub struct WithSpan<T>(pub T);
impl<T: fmt::Display> fmt::Display for WithSpan<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl<T: SyntaxRule> SyntaxRule for WithSpan<T> {
    type Output = Spanned<T::Output>;

    fn prefix_matches(&self, p: Parser<'_>) -> bool {
        self.0.prefix_matches(p)
    }

    fn consume_match(self, p: &mut Parser<'_>) -> ParseResult<Self::Output> {
        let span1 = p.peek_next_span();
        let inner = self.0.consume_match(p)?;
        let span2 = p.span();
        Ok(Spanned::new(Span::merge(span1, span2), inner))
    }
}

/// Rule that matches the same tokens but applies some function to the result.
#[derive(Copy, Clone)]
pub struct MapRule<R, F> {
    /// Inner syntax rule.
    pub inner: R,
    /// Function to apply to the result.
    pub f: F,
}
impl<R: fmt::Debug, F> fmt::Debug for MapRule<R, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MapRule")
            .field("inner", &self.inner)
            .finish()
    }
}
impl<R: fmt::Display, F> fmt::Display for MapRule<R, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}
impl<B, R: SyntaxRule, F: Fn(&mut Parser<'_>, R::Output) -> ParseResult<B>> SyntaxRule
    for MapRule<R, F>
{
    type Output = B;

    fn prefix_matches(&self, p: Parser<'_>) -> bool {
        self.inner.prefix_matches(p)
    }
    fn consume_match(self, p: &mut Parser<'_>) -> ParseResult<Self::Output> {
        let result = self.inner.consume_match(p)?;
        (self.f)(p, result)
    }
}

/// Rule that matches tokens by certain symbols, such as
/// parentheses or brackets.
#[derive(Debug, Clone)]
pub struct Surround<R> {
    /// Inner syntax rule.
    inner: R,

    /// Symbol at start (e.g., left paren).
    start: Token,
    /// Symbol at end (e.g., right paren).
    end: Token,
}
impl<R> Surround<R> {
    fn new(start: Token, inner: R, end: Token) -> Self {
        Self { inner, start, end }
    }

    /// Wraps the rule in parentheses.
    pub fn paren(inner: R) -> Self {
        Self::new(Token::LParen, inner, Token::RParen)
    }
    /// Wraps the rule in brackets.
    pub fn bracket(inner: R) -> Self {
        Self::new(Token::LBracket, inner, Token::RBracket)
    }
    /// Wraps the rule in braces.
    pub fn brace(inner: R) -> Self {
        Self::new(Token::LBrace, inner, Token::RBrace)
    }
}
impl<R: fmt::Display> fmt::Display for Surround<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} surrounded by {} and {}",
            self.inner, self.start, self.end,
        )
    }
}
impl<R: Copy + SyntaxRule> SyntaxRule for Surround<R> {
    type Output = R::Output;

    fn prefix_matches(&self, p: Parser<'_>) -> bool {
        self.start.prefix_matches(p)
    }
    fn consume_match(self, p: &mut Parser<'_>) -> ParseResult<Self::Output> {
        p.parse(&self.start)?;
        let ret = p.parse(self.inner)?;
        p.parse(&self.end)?;
        Ok(ret)
    }
}

/// Rule that matches a list of things surrounded by a symbol pair,
/// such as a comma-separated list enclosed in parentheses.
#[derive(Debug, Clone)]
pub struct List<R> {
    /// Syntax rule for each element of the list.
    pub(super) inner: R,

    /// Separator (e.g., comma).
    pub(super) sep: Token,
    /// Symbol at start (e.g., left paren).
    pub(super) start: Token,
    /// Symbol at end (e.g., right paren).
    pub(super) end: Token,

    /// Whether to allow a trailing separator.
    pub(super) allow_trailing_sep: bool,
    /// Whether to allow an empty list.
    pub(super) allow_empty: bool,
}
impl<R: fmt::Display> fmt::Display for List<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}-separated list of {} surrounded by {} and {}",
            self.sep, self.inner, self.start, self.end,
        )
    }
}
impl<R: SyntaxRule> SyntaxRule for List<R> {
    type Output = Vec<R::Output>;

    fn prefix_matches(&self, p: Parser<'_>) -> bool {
        self.start.prefix_matches(p)
    }
    fn consume_match(self, p: &mut Parser<'_>) -> ParseResult<Self::Output> {
        let mut items = vec![];
        p.parse(&self.start)?;
        loop {
            let may_end_list = match items.is_empty() {
                true => self.allow_empty,
                false => self.allow_trailing_sep,
            };

            let result = if may_end_list {
                // End the list or consume an item.
                parse_one_of!(p, [(&self.end).map(|_| None), (&self.inner).map(Some)])?
            } else {
                // Consume an item.
                parse_one_of!(p, [(&self.inner).map(Some)])?
            };
            match result {
                Some(item) => items.push(item), // There is an item.
                None => break,                  // End of list; empty list, or trailing separator.
            }
            // End the list or consume a separator.
            match parse_one_of!(p, [(&self.end).map(|_| None), (&self.sep).map(Some)])? {
                Some(_) => continue, // There is a separator.
                None => break,       // End of list, no trailing separator.
            }
        }
        Ok(items)
    }
}

/// Rule that matches no tokens; always succeeds (useful as a fallback when
/// matching multiple possible rules).
#[derive(Debug, Copy, Clone)]
pub struct Epsilon;
impl_display!(for Epsilon, "nothing");
impl SyntaxRule for Epsilon {
    type Output = ();

    fn prefix_matches(&self, _p: Parser<'_>) -> bool {
        true
    }
    fn consume_match(self, _p: &mut Parser<'_>) -> ParseResult<Self::Output> {
        Ok(())
    }
}

/// Rule that matches end of file and consumes no tokens.
#[derive(Debug, Copy, Clone)]
pub struct EndOfFile;
impl_display!(for EndOfFile, "end of file");
impl SyntaxRule for EndOfFile {
    type Output = ();

    fn prefix_matches(&self, mut p: Parser<'_>) -> bool {
        *p.next() == Token::Eof
    }
    fn consume_match(self, _p: &mut Parser<'_>) -> ParseResult<Self::Output> {
        Ok(())
    }
}

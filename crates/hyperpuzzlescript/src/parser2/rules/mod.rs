use std::fmt;

pub mod combinators;
pub mod expr;
pub mod literal;
pub mod stmt;

use super::*;

/// A grammar rule that produces an AST node from tokens.
pub trait SyntaxRule: fmt::Debug + fmt::Display {
    /// AST node type that this rule outputs.
    type Output;

    /// Returns whether if it appears that the user is trying to form this
    /// construct (generally returns true if the first token matches). If
    /// `consume_match()` returns `Ok`, this function MUST return true.
    fn prefix_matches(&self, p: Parser<'_>) -> bool;
    /// Consumes the tokens that are part of this syntax structure, returning
    /// the AST node produced. Does NOT restore the `Parser` if matching fails.
    ///
    /// This method may assume that `prefix_matches()` returned `true`. Do not
    /// call this method if it did not.
    fn consume_match(self, p: &mut Parser<'_>) -> ParseResult<Self::Output>;

    /// Applies a function to the output of this syntax rule.
    fn map<B, F: Fn(Self::Output) -> B>(
        self,
        f: F,
    ) -> combinators::MapRule<Self, impl Fn(&mut Parser<'_>, Self::Output) -> ParseResult<B>>
    where
        Self: Sized,
    {
        self.try_map(move |a| Ok(f(a)))
    }
    /// Applies a fallible function to the output of this syntax rule.
    fn try_map<B, F: Fn(Self::Output) -> ParseResult<B>>(
        self,
        f: F,
    ) -> combinators::MapRule<Self, impl Fn(&mut Parser<'_>, Self::Output) -> ParseResult<B>>
    where
        Self: Sized,
    {
        self.and_then(move |_p, a| f(a))
    }
    /// Applies a fallible function to the output of this syntax rule.
    fn and_then<B, F: Fn(&mut Parser<'_>, Self::Output) -> ParseResult<B>>(
        self,
        f: F,
    ) -> combinators::MapRule<Self, F>
    where
        Self: Sized,
    {
        combinators::MapRule { inner: self, f }
    }

    /// Attaches span information to the output.
    fn with_span(self) -> combinators::WithSpan<Self>
    where
        Self: Sized,
    {
        combinators::WithSpan(self)
    }
}

impl<O, T: SyntaxRule<Output = O> + ?Sized> SyntaxRule for Box<T> {
    type Output = O;

    fn prefix_matches(&self, p: Parser<'_>) -> bool {
        self.as_ref().prefix_matches(p)
    }
    fn consume_match(self, p: &mut Parser<'_>) -> ParseResult<Self::Output> {
        self.consume_match(p)
    }
}

// Any `Token` is a `SyntaxRule` that matches only itself.
impl SyntaxRule for Token {
    type Output = Span;

    fn prefix_matches(&self, mut p: Parser<'_>) -> bool {
        p.next() == self
    }
    fn consume_match(self, p: &mut Parser<'_>) -> ParseResult<Self::Output> {
        if *p.next() == self {
            Ok(p.span())
        } else {
            p.expected(self)
        }
    }
}
impl<T: SyntaxRule> SyntaxRule for &T {
    type Output = T::Output;

    fn prefix_matches(&self, p: Parser<'_>) -> bool {
        (*self).prefix_matches(p)
    }
    fn consume_match(self, p: &mut Parser<'_>) -> ParseResult<Self::Output> {
        self.consume_match(p)
    }
}

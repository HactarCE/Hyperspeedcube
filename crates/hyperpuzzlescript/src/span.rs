//! Structs for tracking locations and substrings within strings.

use std::borrow::{Borrow, BorrowMut};
use std::fmt;
use std::ops::{Index, Range};

/// Copyable version of [`Range<u32>`] for storing string indexes.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Span {
    /// The byte index of the first character.
    pub start: u32,
    /// The byte index after the last character.
    pub end: u32,
}
impl From<logos::Span> for Span {
    fn from(value: logos::Span) -> Self {
        Self {
            start: value.start as u32,
            end: value.end as u32,
        }
    }
}
impl Span {
    /// Returns a 0-length span at the given index.
    pub fn empty(idx: u32) -> Self {
        Self {
            start: idx,
            end: idx,
        }
    }
    /// Returns the smallest contiguous span encompassing the two given spans.
    pub fn merge<T: Into<Span>, U: Into<Span>>(span1: T, span2: U) -> Self {
        let span1: Span = span1.into();
        let span2: Span = span2.into();
        Self {
            start: std::cmp::min(span1.start, span2.start),
            end: std::cmp::max(span1.end, span2.end),
        }
    }
    /// Returns the equivalent [`Range<usize>`].
    pub fn range(self) -> Range<usize> {
        self.into()
    }
    /// Returns the substring with this span from a string.
    pub fn of<S: Index<Range<usize>>>(self, s: &S) -> &S::Output {
        &s[self.range()]
    }
}
impl<T> From<Spanned<T>> for Span {
    fn from(spanned: Spanned<T>) -> Self {
        spanned.span
    }
}
impl<T> From<&Spanned<T>> for Span {
    fn from(spanned: &Spanned<T>) -> Self {
        spanned.span
    }
}
impl From<&Span> for Span {
    fn from(span: &Span) -> Self {
        *span
    }
}
impl From<[u32; 2]> for Span {
    fn from(span: [u32; 2]) -> Self {
        Self {
            start: span[0],
            end: span[1],
        }
    }
}
impl From<Span> for Range<usize> {
    fn from(val: Span) -> Self {
        let Span { start, end } = val;
        start as usize..end as usize
    }
}

/// Any data with an associated span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spanned<T> {
    /// The span.
    pub span: Span,
    /// The data.
    pub inner: T,
}
impl<T> Borrow<T> for Spanned<T> {
    fn borrow(&self) -> &T {
        &self.inner
    }
}
impl<T> BorrowMut<T> for Spanned<T> {
    fn borrow_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}
impl<T: fmt::Display> fmt::Display for Spanned<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}
impl<T, S: Into<Span>> From<(T, S)> for Spanned<T> {
    fn from(value: (T, S)) -> Self {
        Spanned::new(value.1, value.0)
    }
}
impl<T> Spanned<T> {
    /// Constructs a `Spanned<T>` spanning the given byte indices.
    pub fn new(span: impl Into<Span>, inner: T) -> Self {
        let span = span.into();
        Self { span, inner }
    }
    /// Applies a function to the inside of a `Spanned<T>`.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Spanned<U> {
        Spanned {
            span: self.span,
            inner: f(self.inner),
        }
    }
    /// Converts a `&Spanned<T>` to a `Spanned<&T>`.
    pub fn as_ref(&self) -> Spanned<&T> {
        Spanned {
            span: self.span,
            inner: &self.inner,
        }
    }

    /// Returns the equivalent `Range<usize>` for the span.
    pub fn range(&self) -> Range<usize> {
        self.span.range()
    }

    /// Merges two spans using `Span::merge()` and merges the inner values using
    /// the provided function.
    pub fn merge<U, V>(a: Spanned<U>, b: Spanned<V>, merge: impl FnOnce(U, V) -> T) -> Spanned<T> {
        Spanned {
            span: Span::merge(a.span, b.span),
            inner: merge(a.inner, b.inner),
        }
    }
}
impl<T, E> Spanned<Result<T, E>> {
    /// Converts a `Spanned<Result<T, E>>` to a `Result<Spanned<T>, E>`, losing
    /// the span information in the error case.
    pub fn transpose(self) -> Result<Spanned<T>, E> {
        let span = self.span;
        match self.inner {
            Ok(inner) => Ok(Spanned { span, inner }),
            Err(e) => Err(e),
        }
    }
}

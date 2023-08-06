//! Common utility functions.

use itertools::Itertools;
use tinyset::Set64;

/// Extension trait for `.fold_intersection()`.
pub trait Set64IntersectionIterExt {
    /// Output of `.fold_intersection()`.
    type Output;

    /// Returns the intersection of all the sets produced by an iterator.
    fn fold_intersection(self) -> Self::Output;
}
impl<'a, I, T: 'a> Set64IntersectionIterExt for I
where
    I: Iterator<Item = &'a Set64<T>>,
    T: tinyset::Fits64,
{
    type Output = Set64<T>;

    fn fold_intersection(mut self) -> Set64<T> {
        let mut ret = self.next().unwrap_or(&Set64::new()).clone();
        for it in self {
            ret = set64_intersection(&ret, it);
        }
        ret
    }
}
/// Extension trait for `.try_fold_intersection()`.
pub trait Set64TryIntersectionIterExt {
    /// Output of `.try_fold_intersection()`.
    type Output;

    /// Returns the intersection of all the sets produced by an iterator, or an
    /// error if any element is `Err`.
    fn try_fold_intersection(self) -> Self::Output;
}
impl<'a, I, T: 'a, E> Set64TryIntersectionIterExt for I
where
    I: Iterator<Item = Result<&'a Set64<T>, E>>,
    T: tinyset::Fits64,
{
    type Output = Result<Set64<T>, E>;

    fn try_fold_intersection(mut self) -> Result<Set64<T>, E> {
        let mut ret = self.next().unwrap_or(Ok(&Set64::new()))?.clone();
        for it in self {
            ret = set64_intersection(&ret, it?);
        }
        Ok(ret)
    }
}

/// Returns whether `a` is a subset of `b`.
pub fn is_subset<T: tinyset::Fits64>(a: &Set64<T>, b: &Set64<T>) -> bool {
    a.iter().all(|elem| b.contains(elem))
}

/// Returns the intersection of `a` and `b`.
pub fn set64_intersection<T: tinyset::Fits64>(a: &Set64<T>, b: &Set64<T>) -> Set64<T> {
    a.iter().filter(|elem| b.contains(elem)).collect()
}

/// Returns an iterator over uppercase letter sequences.
pub fn letters_upper() -> impl Iterator<Item = String> {
    (1..).flat_map(|num_chars| {
        std::iter::repeat('A'..='Z')
            .take(num_chars)
            .multi_cartesian_product()
            .map(|chars| chars.into_iter().collect())
    })
}
/// Returns an iterator over lowercase letter sequences.
pub fn letters_lower() -> impl Iterator<Item = String> {
    (1..).flat_map(|num_chars| {
        std::iter::repeat('a'..='z')
            .take(num_chars)
            .multi_cartesian_product()
            .map(|chars| chars.into_iter().collect())
    })
}

/// If both options are `Some`, merges them using `f`. Otherwise returns
/// whichever one is `Some`, or `None` if they are both `None`.
pub fn merge_options<T>(a: Option<T>, b: Option<T>, f: impl FnOnce(T, T) -> T) -> Option<T> {
    match (a, b) {
        (Some(a), Some(b)) => Some(f(a, b)),
        (a, b) => a.or(b),
    }
}

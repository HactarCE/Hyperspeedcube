//! Math utility functions.

use std::ops::{Add, Mul};

use super::Float;

/// Linearly interpolates (unclamped) between two values.
pub fn mix<T>(a: T, b: T, t: Float) -> <T::Output as Add>::Output
where
    T: Mul<Float>,
    T::Output: Add,
{
    a * (1.0 - t) + b * t
}

/// Returns the element of an iterator with the minimum Float value.
pub fn min_by_float_key<T>(
    elems: impl IntoIterator<Item = T>,
    mut f: impl FnMut(&T) -> Float,
) -> Option<T> {
    let mut iter = elems.into_iter();
    let mut min_elem = iter.next()?;
    let mut min_key = f(&min_elem);
    for elem in iter {
        let key = f(&elem);
        if key < min_key {
            min_elem = elem;
            min_key = key;
        }
    }
    Some(min_elem)
}

/// Divides `lhs` by `rhs` if the reciprocal of `rhs` is finite; otherwise
/// returns `None`.
pub fn try_div<T>(lhs: T, rhs: Float) -> Option<T::Output>
where
    T: Mul<Float>,
{
    let recip_rhs = rhs.recip();
    recip_rhs.is_finite().then(|| lhs * recip_rhs)
}

/// Returns the square root of `n` if the result is finite; otherwise returns
/// `None`.
pub fn try_sqrt(n: Float) -> Option<Float> {
    let ret = n.sqrt();
    ret.is_finite().then_some(ret)
}

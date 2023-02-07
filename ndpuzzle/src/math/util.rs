//! Math utility functions.

use std::ops::{Add, Mul};

/// Linearly interpolates (unclamped) between two values.
pub fn mix<T>(a: T, b: T, t: f32) -> <T::Output as Add>::Output
where
    T: Mul<f32>,
    T::Output: Add,
{
    a * (1.0 - t) + b * t
}

/// Returns the element of an iterator with the minimum f32 value.
pub fn min_by_f32_key<T>(
    elems: impl IntoIterator<Item = T>,
    mut f: impl FnMut(&T) -> f32,
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

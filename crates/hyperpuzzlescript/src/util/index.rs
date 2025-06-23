use crate::{Error, Result, Spanned};

/// Indexes a collection.
///
/// - `get_front` must return the `i`th value from the front of the collection
///   (starting at zero)
/// - `get_back` must return the `i`th value from the back of the collection
///   (starting at zero).
/// - `get_back` must be `None` if the collection does not support indexing from
///   the back.
/// - `get_len` must return the length of the collection.
pub fn index<C, T>(
    collection: C,
    get_front: fn(C, usize) -> Option<T>,
    get_back: Option<fn(C, usize) -> Option<T>>,
    get_len: impl FnOnce() -> usize,
    (idx, idx_span): Spanned<i64>,
) -> Result<T> {
    let allow_negatives = get_back.is_some();
    match idx {
        0.. => get_front(collection, idx.try_into().unwrap_or(usize::MAX)),
        ..0 => get_back.and_then(|f| f(collection, (-idx - 1).try_into().unwrap_or(usize::MAX))),
    }
    .ok_or_else(|| {
        Error::IndexOutOfBounds {
            got: idx,
            bounds: (|| {
                let max = get_len().checked_sub(1)? as i64;
                let min = if allow_negatives { -max - 1 } else { 0 };
                Some((min, max))
            })(),
        }
        .at(idx_span)
    })
}

/// Indexes a double-ended iterator.
///
/// This may be take O(n) time with respect to the size of the collection, but
/// many double-ended iterators in Rust have O(1) implementations of `.nth()`
/// and `.nth_back()` so it is often performant.
pub(crate) fn index_double_ended<I: IntoIterator>(
    iter: I,
    get_len: impl FnOnce() -> usize,
    idx: Spanned<i64>,
) -> Result<I::Item>
where
    I::IntoIter: DoubleEndedIterator,
{
    index(
        iter.into_iter(),
        |mut it: I::IntoIter, i| it.nth(i),
        Some(|mut it: I::IntoIter, i| it.nth_back(i)),
        get_len,
        idx,
    )
}

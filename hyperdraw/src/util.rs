/// Iterator over adjacent pairs in a cycle. See
/// [`IterCyclicPairsExt::cyclic_pairs()`].
pub struct CyclicPairsIter<I: Iterator> {
    first: Option<I::Item>,
    prev: Option<I::Item>,
    rest: I,
}
impl<I> Iterator for CyclicPairsIter<I>
where
    I: Iterator,
    I::Item: Clone,
{
    type Item = (I::Item, I::Item);

    fn next(&mut self) -> Option<Self::Item> {
        Some(match self.rest.next() {
            Some(curr) => (self.prev.replace(curr.clone())?, curr),
            None => (self.prev.take()?, self.first.take()?),
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (lo, hi) = self.rest.size_hint();
        (lo.saturating_add(1), hi.and_then(|x| x.checked_add(1)))
    }
}

/// Extension trait for `.cyclic_pairs()`.
pub trait IterCyclicPairsExt: Iterator + Sized {
    /// Returns an iterator over adjacent pairs in a cycle.
    ///
    /// # Example
    ///
    /// ```
    /// # use hyperdraw::IterCyclicPairsExt;
    ///
    /// let mut iter = [1, 2, 3, 4].into_iter().cyclic_pairs();
    /// assert_eq!(Some((1, 2)), iter.next());
    /// assert_eq!(Some((2, 3)), iter.next());
    /// assert_eq!(Some((3, 4)), iter.next());
    /// assert_eq!(Some((4, 1)), iter.next());
    /// assert_eq!(None, iter.next());
    /// ```
    fn cyclic_pairs(self) -> CyclicPairsIter<Self>;
}
impl<I> IterCyclicPairsExt for I
where
    I: Iterator,
    I::Item: Clone,
{
    fn cyclic_pairs(mut self) -> CyclicPairsIter<Self> {
        let first = self.next();
        let prev = first.clone();
        CyclicPairsIter {
            first,
            prev,
            rest: self,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cyclic_pairs_iter() {
        assert_eq!(
            [1, 2, 3, 4].into_iter().cyclic_pairs().collect::<Vec<_>>(),
            vec![(1, 2), (2, 3), (3, 4), (4, 1)],
        );
        assert_eq!(
            [1, 2, 3].into_iter().cyclic_pairs().collect::<Vec<_>>(),
            vec![(1, 2), (2, 3), (3, 1)],
        );
        assert_eq!(
            [1, 2].into_iter().cyclic_pairs().collect::<Vec<_>>(),
            vec![(1, 2), (2, 1)],
        );
        assert_eq!(
            [1].into_iter().cyclic_pairs().collect::<Vec<_>>(),
            vec![(1, 1)],
        );
    }
}

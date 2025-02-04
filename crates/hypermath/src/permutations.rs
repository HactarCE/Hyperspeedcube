//! Permutation math.

use itertools::Itertools;

/// Parity of a permutation.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(i8)]
pub enum Parity {
    /// Even number of swaps.
    #[default]
    Even = 0,
    /// Odd number of swaps.
    Odd = 1,
}
impl Parity {
    /// Returns the opposite parity.
    #[must_use]
    pub fn opposite(self) -> Self {
        match self {
            Parity::Even => Parity::Odd,
            Parity::Odd => Parity::Even,
        }
    }
}

/// Returns an iterator over permutations of a list, each with its associated
/// parity.
pub fn permutations_with_parity<I>(iter: I) -> impl Iterator<Item = (Vec<I::Item>, Parity)>
where
    I: ExactSizeIterator,
    I::Item: Clone,
{
    let len = iter.len();
    iter.permutations(len)
        .enumerate()
        .map(|(i, p)| (p, permutation_parity(i)))
}

/// Returns the parity of the permutation with number `n`.
pub fn permutation_parity(mut n: usize) -> Parity {
    let mut res = Parity::Even;
    let mut i = 2;
    while n > 0 {
        if (n % i) % 2 != 0 {
            res = res.opposite();
        }
        n /= i;
        i += 1;
    }
    res
}

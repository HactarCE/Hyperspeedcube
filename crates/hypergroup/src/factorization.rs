use smallvec::SmallVec;

use super::*;

/// Factorization of a group element into generators.
#[derive(Debug, Default, Clone)]
pub struct Factorization<'a>(SmallVec<[&'a [GeneratorId]; 8]>);

impl Factorization<'_> {
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        self.0.iter().map(|v| v.len()).sum()
    }
}

impl<'a> IntoIterator for Factorization<'a> {
    type Item = GeneratorId;

    type IntoIter = FactorizationIntoIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        FactorizationIntoIter(self.0.into_iter().flatten())
    }
}

impl<'a> From<&'a [GeneratorId]> for Factorization<'a> {
    fn from(value: &'a [GeneratorId]) -> Self {
        Self::from_iter([value])
    }
}

impl<'a> FromIterator<&'a [GeneratorId]> for Factorization<'a> {
    fn from_iter<T: IntoIterator<Item = &'a [GeneratorId]>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<'a> FromIterator<Factorization<'a>> for Factorization<'a> {
    fn from_iter<T: IntoIterator<Item = Factorization<'a>>>(iter: T) -> Self {
        Self(iter.into_iter().flat_map(|f| f.0).collect())
    }
}

/// Iterator over a group element's factorization.
#[derive(Clone)]
pub struct FactorizationIntoIter<'a>(
    std::iter::Flatten<smallvec::IntoIter<[&'a [GeneratorId]; 8]>>,
);

impl Iterator for FactorizationIntoIter<'_> {
    type Item = GeneratorId;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().copied()
    }
}

impl DoubleEndedIterator for FactorizationIntoIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back().copied()
    }
}

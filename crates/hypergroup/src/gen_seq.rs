use smallvec::SmallVec;

use crate::GeneratorId;

/// Abbreviated generator sequence; generator sequence that may be expressed in
/// terms of another element in the orbit.
#[derive(Debug, Default, Clone)]
pub struct AbbrGenSeq {
    /// Generator indices.
    pub generators: GenSeq,
    /// Index of an optional final element, whose generators should be applied
    /// after `generators`.
    pub end: Option<usize>,
}
impl AbbrGenSeq {
    /// The empty generator sequence, which identifies the initial element in an
    /// orbit.
    pub const INIT: Self = Self {
        generators: GenSeq::INIT,
        end: None,
    };

    /// Constructs a new abbreviated generator sequence that consists of a
    /// sequence of indices followed by the generator sequence of `end`.
    pub fn new(indices: impl IntoIterator<Item = GeneratorId>, end: Option<usize>) -> Self {
        let generators = GenSeq::new(indices);
        AbbrGenSeq { generators, end }
    }
}

/// Generator sequence to reach an element in an orbit.
#[derive(Debug, Default, Clone)]
pub struct GenSeq(pub SmallVec<[GeneratorId; 8]>);
impl GenSeq {
    /// The empty generator sequence, which identifies the initial element in an
    /// orbit.
    pub const INIT: Self = Self(SmallVec::new_const());

    /// Constructs a new generator sequence.
    pub fn new(indices: impl IntoIterator<Item = GeneratorId>) -> Self {
        Self::from_iter(indices)
    }
}
impl FromIterator<GeneratorId> for GenSeq {
    fn from_iter<T: IntoIterator<Item = GeneratorId>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

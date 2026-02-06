use hypermath::prelude::*;
use hyperpuzzle_util::ti::{IndexOverflow, TiVec, TypedIndex};

use super::{AbbrGenSeq, GenSeq};

/// Returns the orbit of an object under the symmetry. Each generator is
/// specified along with its generator sequence.
pub fn orbit<T: Clone + ApproxHash + Ndim + TransformByMotor>(
    generators: &[(GenSeq, pga::Motor)],
    mut object: T,
) -> Vec<(AbbrGenSeq, pga::Motor, T)> {
    let ndim = generators
        .iter()
        .map(|(_, m)| m.ndim())
        .max()
        .unwrap_or(1)
        .max(object.ndim());

    let mut seen = ApproxHashMap::new(APPROX);
    seen.entry_with_mut_key(&mut object).or_insert(());

    let mut next_unprocessed_index = 0;
    let mut ret = vec![(AbbrGenSeq::INIT, pga::Motor::ident(ndim), object)];
    while next_unprocessed_index < ret.len() {
        let (_gen_seq, unprocessed_transform, unprocessed_object) =
            ret[next_unprocessed_index].clone();
        for (gen_seq_ids, generator) in generators {
            let mut new_object = generator.transform(&unprocessed_object);
            if let approx_collections::hash_map::Entry::Vacant(e) =
                seen.entry_with_mut_key(&mut new_object)
            {
                e.insert(());
                let gen_seq = AbbrGenSeq {
                    generators: gen_seq_ids.clone(),
                    end: Some(next_unprocessed_index),
                };
                ret.push((gen_seq, generator * &unprocessed_transform, new_object));
            }
        }
        next_unprocessed_index += 1;
    }
    ret
}

hyperpuzzle_util::typed_index_struct! {
    /// ID of a group generator.
    pub struct GeneratorId(pub u8);
    /// ID of a group element.
    pub struct GroupElementId(pub u16);
}
impl From<GeneratorId> for GroupElementId {
    fn from(value: GeneratorId) -> Self {
        GroupElementId(value.0 as u16 + 1)
    }
}
impl GroupElementId {
    /// Identity element in any group.
    pub const IDENTITY: GroupElementId = GroupElementId(0);
}

/// List containing a value per group generator.
pub type PerGenerator<T> = TiVec<GeneratorId, T>;
/// List containing a value per group element.
pub type PerGroupElement<T> = TiVec<GroupElementId, T>;

/// Error that can occur during group construction.
#[allow(missing_docs)]
#[derive(thiserror::Error, Debug, Clone)]
pub enum GroupError {
    #[error("invalid group generator {0}")]
    InvalidGenerator(pga::Motor),
    #[error("overflow ({0})")]
    Overflow(IndexOverflow),
    #[error("group is too high-dimensional")]
    TooHighDimensional,

    #[error("missing inverse for element {0}")]
    MissingInverse(GroupElementId),
    #[error("missing successor for element {0} and generator {1}")]
    MissingSuccessor(GroupElementId, GeneratorId),
    #[error("incomplete group structure")]
    IncompleteGroupStructure,

    #[error("bad group structure")]
    BadGroupStructure,
    #[error("bad inverse; inverse of {0} is {1} but inverse of {1} is {2}")]
    BadInverse(GroupElementId, GroupElementId, GroupElementId),

    #[error("coxeter-dynkin diagram is hyperbolic")]
    HyperbolicCD,
    #[error("coxeter-dynkin diagram is euclidean")]
    EuclideanCD,
    #[error("invalid coxeter-dynkin diagram")]
    BadCD,

    #[error("invalid coxeter-dynkin basis")]
    BadBasis,
}
impl From<IndexOverflow> for GroupError {
    fn from(value: IndexOverflow) -> Self {
        GroupError::Overflow(value)
    }
}

/// Result type returned by group construction operations.
pub type GroupResult<T> = Result<T, GroupError>;

/// Element-generator group table.
///
/// 2D array containing a value for each possible element+generator pairing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct EggTable<T> {
    /// Number of elements in the group.
    element_count: usize,
    /// Number of generators in the group.
    generator_count: usize,
    /// Flattened 2D array, indexed by a pair of element ID and value index.
    contents: Vec<T>,
}
impl<T: Default + Clone> EggTable<T> {
    /// Constructs a new EGG table containing only the identity.
    pub fn new(generator_count: usize) -> Self {
        EggTable {
            element_count: 1,
            generator_count,
            contents: vec![T::default(); generator_count],
        }
    }
    /// Adds a new element to the table.
    pub fn add_element(&mut self, value: T) -> GroupResult<GroupElementId> {
        let new_element = GroupElementId::try_from_index(self.element_count)?;
        self.element_count += 1;
        self.contents
            .extend(std::iter::repeat_n(value, self.generator_count));
        Ok(new_element)
    }
    /// Returns a value from the table.
    #[inline]
    #[track_caller]
    pub fn get(&self, element: GroupElementId, generator: GeneratorId) -> &T {
        &self.contents[self.index(element, generator)]
    }
    /// Returns a mutable reference to a value in the table.
    #[track_caller]
    pub fn get_mut(&mut self, element: GroupElementId, generator: GeneratorId) -> &mut T {
        let index = self.index(element, generator);
        &mut self.contents[index]
    }

    /// Returns an integer index into `contents`.
    #[inline]
    #[track_caller]
    fn index(&self, element: GroupElementId, generator: GeneratorId) -> usize {
        assert!(
            (generator.0 as usize) < self.generator_count,
            "generator {generator} out of range (max {max})",
            max = self.generator_count,
        );
        element.0 as usize * self.generator_count + generator.0 as usize
    }

    /// Returns an iterator over keys and values.
    pub fn iter(&self) -> impl '_ + Iterator<Item = ((GroupElementId, GeneratorId), &T)> {
        let elements_iter = GroupElementId::iter(self.element_count);
        let generators_iter = GeneratorId::iter(self.generator_count);
        itertools::iproduct!(elements_iter, generators_iter).zip(&self.contents)
    }
}
impl<T> EggTable<Option<T>> {
    pub fn try_unwrap(self) -> GroupResult<EggTable<T>> {
        match self.contents.into_iter().collect::<Option<Vec<T>>>() {
            Some(contents) => Ok(EggTable {
                element_count: self.element_count,
                generator_count: self.generator_count,
                contents,
            }),
            None => Err(GroupError::IncompleteGroupStructure),
        }
    }
}
impl EggTable<GroupElementId> {
    /// Performs basic sanity checks on this table, assuming it is intended to
    /// be a table of successors, and returns an error if it does not make sense
    /// for a group.
    ///
    /// Invalid groups may pass these checks.
    pub fn sanity_check_successors(&self) -> GroupResult<()> {
        let mut counts: PerGroupElement<usize> = (0..self.element_count).map(|_| 0).collect();

        for ((elem, generator), &successor) in self.iter() {
            let mut ok = true;

            // Applying a generator should produce a new element.
            ok &= elem != successor;

            // Only the identity has each generator as its own corresponding
            // successor.
            let is_identity = elem == GroupElementId::IDENTITY;
            ok &= is_identity == (successor == GroupElementId::from(generator));

            if !ok {
                return Err(GroupError::BadGroupStructure);
            }
            counts[successor] += 1;
        }

        // Check that every element has the same number of occurrences in the
        // successor table.
        for &count in counts.iter_values() {
            if count != self.generator_count {
                return Err(GroupError::BadGroupStructure);
            }
        }

        Ok(())
    }
}

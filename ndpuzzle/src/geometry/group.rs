use anyhow::{ensure, Context, Result};
use parking_lot::{Condvar, Mutex};
use smallvec::{smallvec, SmallVec};
use std::ops::Index;
use std::sync::Arc;

use crate::collections::{
    approx_hashmap, GenericVec, IndexNewtype, IsometryHashMap, IsometryNearestNeighborsMap,
};
use crate::math::cga::*;

idx_struct! {
    /// ID of a group element.
    pub struct GeneratorId(u8);
    /// ID of a group element.
    pub struct ElementId(u16);
}
impl From<GeneratorId> for ElementId {
    fn from(value: GeneratorId) -> Self {
        ElementId(value.0 as u16 + 1)
    }
}
impl ElementId {
    /// Identity element in any group.
    pub const IDENTITY: ElementId = ElementId(0);
}

type PerGenerator<T> = GenericVec<GeneratorId, T>;
type PerElement<T> = GenericVec<ElementId, T>;

/// Isometry group in some number of dimensions.
#[derive(Debug, Clone)]
pub struct IsometryGroup {
    /// Number of generators in the group. If there are `N` generators, then the
    /// generators are always elements `1..=N`.
    generator_count: usize,

    /// Elements of the group, indexed by ID.
    elements: PerElement<Isometry>,

    /// Generator sequences that produce each element.
    generator_sequences: PerElement<SmallVec<[GeneratorId; 16]>>,
    /// Element inverses.
    inverses: PerElement<ElementId>,
    /// Results of multiplying an element by a generator.
    successors: PerGenerator<PerElement<ElementId>>,

    /// Nearest neighbors data structure.
    nearest_neighbors: IsometryNearestNeighborsMap<ElementId>,
}

impl Default for IsometryGroup {
    fn default() -> Self {
        Self::from_generators(&[]).unwrap()
    }
}

impl Index<ElementId> for IsometryGroup {
    type Output = Isometry;

    fn index(&self, index: ElementId) -> &Self::Output {
        &self.elements[index]
    }
}

impl Index<GeneratorId> for IsometryGroup {
    type Output = Isometry;

    fn index(&self, index: GeneratorId) -> &Self::Output {
        &self.elements[index.into()]
    }
}

impl IsometryGroup {
    /// Construct a group from a set of generators.
    pub fn from_generators(generators: &[Isometry]) -> Result<Self> {
        let generator_count = generators.len();
        let generators = generators
            .iter()
            .map(Isometry::canonicalize)
            .collect::<Option<PerGenerator<Isometry>>>()
            .context("invalid group generator")?;

        let mut elements = PerElement::from_iter([Isometry::ident()]);
        let mut element_ids = IsometryHashMap::new();
        element_ids.insert_canonicalized(&Isometry::ident(), ElementId::IDENTITY);

        let mut generator_sequences = PerElement::from_iter([smallvec![]]);
        let mut successors = generators.map(|_| PerElement::new());

        // Computing inverses directly is doable, but might involve a lot of
        // floating-point math. Instead, keep track of the inverse of each
        // generator.
        let mut generator_inverses =
            PerGenerator::from_iter((0..generator_count).map(|_| ElementId::IDENTITY));

        rayon::scope(|s| -> Result<()> {
            // Use `elements` as a queue. Keep pushing elements onto the end of
            // it, and "popping" them off the front by moving
            // `next_unprocessed_id` forward.
            let mut next_unprocessed_id = ElementId::IDENTITY;
            let mut unprocessed_successors =
                PerElement::from_iter([Arc::new(Task::new_already_computed(generators.clone()))]);
            while (next_unprocessed_id.0 as usize) < elements.len() {
                let initial_gen_seq = generator_sequences[next_unprocessed_id].clone();

                // Get the result of applying each generator to
                // `next_unprocessed`.
                let successors_to_process =
                    unprocessed_successors[next_unprocessed_id].block_on_result();

                // Apply each generator to `next_unprocessed` and see where it
                // goes.
                for (gen, new_elem) in successors_to_process.into_iter() {
                    let id;
                    match element_ids.entry_canonicalized(&new_elem) {
                        // We've already seen `new_elem`.
                        approx_hashmap::Entry::Occupied(e) => {
                            id = *e.into_mut();

                            if id == ElementId::IDENTITY {
                                // We multiplied `next_unprocessed * gen` and
                                // got the identity element, so
                                // `next_unprocessed` and `gen` must be
                                // inverses.
                                generator_inverses[gen] = next_unprocessed_id;
                            }
                        }

                        // `new_elem` has never been seen before. Assign it a
                        // new ID and add it to all the relevant lists.
                        approx_hashmap::Entry::Vacant(e) => {
                            id = elements.push(new_elem.clone())?;

                            // Enqueue a new task to compute the successors of
                            // `new_elem`.
                            let task = Arc::new(Task::new());
                            unprocessed_successors.push(Arc::clone(&task))?;
                            let generators_ref = &generators;
                            s.spawn(move |_| {
                                task.store(generators_ref.map(|(_id, gen)| {
                                    (&new_elem * gen).canonicalize().unwrap_or_default()
                                }))
                            });

                            e.insert(id);

                            let mut gen_seq = initial_gen_seq.clone();
                            gen_seq.push(gen);
                            generator_sequences.push(gen_seq)?;
                        }
                    }
                    // Record the result of `new_elem * gen`.
                    successors[gen].push(id)?;
                }
                next_unprocessed_id.0 += 1;
            }
            // We've applyied every generator to every element, so we've
            // generated the whole group.

            Ok(())
        })?;

        ensure!(elements.len() == generator_sequences.len());
        if generator_count > 0 {
            ensure!(elements.len() == successors[GeneratorId(0)].len());
        }

        let nearest_neighbors =
            IsometryNearestNeighborsMap::new(&elements, elements.iter_keys().collect());

        let mut this = Self {
            generator_count,

            elements,

            generator_sequences,
            inverses: PerElement::new(),
            successors,

            nearest_neighbors,
        };

        // Compute inverses.
        this.inverses = this
            .elements()
            .map(|initial_element| {
                this.generator_sequence(initial_element)
                    .iter()
                    .rev()
                    .map(|&g| generator_inverses[g])
                    .reduce(|elem, inv_gen| this.compose(elem, inv_gen))
                    .unwrap_or(ElementId::IDENTITY)
            })
            .collect();

        for element in this.elements() {
            ensure!(this.inverse(this.inverse(element)) == element);
        }

        Ok(this)
    }

    /// Returns the number of generators used to generate the group.
    pub fn generator_count(&self) -> usize {
        self.generator_count
    }
    /// Returns the number of elements in the group.
    pub fn element_count(&self) -> usize {
        self.elements.len()
    }

    /// Returns an iterator over the generators used to generate the group.
    pub fn generators(&self) -> impl Iterator<Item = GeneratorId> {
        GeneratorId::iter(self.generator_count())
    }
    /// Returns an iterator over the elements of the group.
    pub fn elements(&self) -> impl Iterator<Item = ElementId> {
        ElementId::iter(self.element_count())
    }

    /// Returns the shortest sequence of generators that multiplies to produce
    /// an element. Ties are broken by lexicographical ordering.
    pub fn generator_sequence(&self, a: ElementId) -> &[GeneratorId] {
        &self.generator_sequences[a]
    }
    /// Returns the inverse element.
    pub fn inverse(&self, a: ElementId) -> ElementId {
        self.inverses[a]
    }

    /// Composes two elements.
    pub fn compose(&self, a: ElementId, b: ElementId) -> ElementId {
        self.compose_gen_seq(a, self.generator_sequence(b))
    }
    /// Composes an element with a sequence of generators.
    fn compose_gen_seq(&self, a: ElementId, b: &[GeneratorId]) -> ElementId {
        b.iter().fold(a, |p, &q| self.successor(p, q))
    }

    /// Composes an element with a generator.
    pub fn successor(&self, a: ElementId, b: GeneratorId) -> ElementId {
        self.successors[b][a]
    }

    /// Returns the nearest element.
    pub fn nearest(&self, target: &Isometry) -> ElementId {
        match self.nearest_neighbors.nearest(target) {
            Some(&e) => e,
            None => ElementId::IDENTITY,
        }
    }
}

/// One-time computation task for a worker thread.
#[derive(Debug, Default)]
struct Task<T> {
    condvar: Condvar,
    result: Mutex<Option<T>>,
}
impl<T> Task<T> {
    fn new_already_computed(value: T) -> Self {
        Self {
            condvar: Condvar::new(),
            result: Mutex::new(Some(value)),
        }
    }
    fn new() -> Self {
        Self {
            condvar: Condvar::new(),
            result: Mutex::new(None),
        }
    }
    fn store(&self, value: T) {
        *self.result.lock() = Some(value);
        self.condvar.notify_one();
    }
    fn block_on_result(&self) -> T {
        let mut mutex_guard = self.result.lock();
        loop {
            match mutex_guard.take() {
                Some(result) => return result,
                None => self.condvar.wait(&mut mutex_guard),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::geometry::SchlafliSymbol;
    use crate::math::*;

    #[test]
    fn test_cyclic_groups() {
        fn cyclic_group(n: Float) -> IsometryGroup {
            IsometryGroup::from_generators(&[Isometry::from_angle_in_normalized_plane(
                Vector::unit(0),
                Vector::unit(1),
                std::f64::consts::PI as Float * 2.0 / n,
            )])
            .unwrap()
        }

        assert_eq!(5, cyclic_group(5.0).element_count());

        assert_eq!(7, cyclic_group(7.0 / 2.0).element_count());
    }

    #[test]
    fn test_cube_group() {
        let g = SchlafliSymbol::from_indices(vec![4, 3]).group().unwrap();

        assert_eq!(48, g.element_count());
    }

    #[test]
    fn test_120cell_group() {
        let g = SchlafliSymbol::from_indices(vec![5, 3, 3]).group().unwrap();

        assert_eq!(14400, g.element_count());
    }
}

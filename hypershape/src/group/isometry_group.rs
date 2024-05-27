use std::ops::Index;
use std::sync::Arc;

use hypermath::collections::{approx_hashmap, ApproxHashMap, MotorNearestNeighborMap};
use hypermath::prelude::*;
use itertools::Itertools;
use parking_lot::{Condvar, Mutex};

use super::{
    AbstractGroup, GeneratorId, Group, GroupBuilder, GroupElementId, GroupError, GroupResult,
    PerGroupElement, PerGenerator,
};

/// Discrete subgroup of the [isometry group](https://w.wiki/7QFZ) of a space.
#[derive(Debug, Clone)]
pub struct IsometryGroup {
    /// Underlying group structure.
    group: AbstractGroup,
    /// Elements of the group, indexed by ID.
    elements: PerGroupElement<pga::Motor>,
    /// Nearest neighbors data structure.
    nearest_neighbors: MotorNearestNeighborMap<GroupElementId>,
}

impl Default for IsometryGroup {
    fn default() -> Self {
        Self::from_generators(&[]).expect("failed to construct trivial group")
    }
}

impl Group for IsometryGroup {
    fn group(&self) -> &AbstractGroup {
        &self.group
    }
}

impl Index<GroupElementId> for IsometryGroup {
    type Output = pga::Motor;

    fn index(&self, index: GroupElementId) -> &Self::Output {
        &self.elements[index]
    }
}

impl Index<GeneratorId> for IsometryGroup {
    type Output = pga::Motor;

    fn index(&self, index: GeneratorId) -> &Self::Output {
        &self.elements[index.into()]
    }
}

impl IsometryGroup {
    /// Construct a group from a set of generators.
    pub fn from_generators(generators: &[pga::Motor]) -> GroupResult<Self> {
        let generator_count = generators.len();

        let mut g = GroupBuilder::new(generator_count)?;

        let ndim = generators.iter().map(|gen| gen.ndim()).max().unwrap_or(2);

        let generators: PerGenerator<pga::Motor> = generators
            .iter()
            .map(|motor| {
                motor
                    .to_ndim_at_least(ndim)
                    .canonicalize()
                    .ok_or_else(|| GroupError::InvalidGenerator(motor.clone()))
            })
            .try_collect()?;

        let mut elements = PerGroupElement::from_iter([pga::Motor::ident(ndim)]);
        let mut element_ids = ApproxHashMap::new();
        element_ids.insert(pga::Motor::ident(ndim), GroupElementId::IDENTITY);

        // Computing inverses directly is doable, but might involve a lot of
        // floating-point math. Instead, keep track of the inverse of each
        // generator.
        let mut generator_inverses =
            PerGenerator::from_iter((0..generator_count).map(|_| GroupElementId::IDENTITY));

        rayon::scope(|s| -> GroupResult<()> {
            // Use `elements` as a queue. Keep pushing elements onto the end of
            // it, and "popping" them off the front by moving
            // `next_unprocessed_id` forward.
            let mut next_unprocessed_id = GroupElementId::IDENTITY;
            let mut unprocessed_successors =
                PerGroupElement::from_iter([Arc::new(Task::new_already_computed(generators.clone()))]);
            while (next_unprocessed_id.0 as usize) < elements.len() {
                // Get the result of applying each generator to
                // `next_unprocessed`.
                let successors_to_process =
                    unprocessed_successors[next_unprocessed_id].block_on_result();

                // Apply each generator to `next_unprocessed` and see where it
                // goes.
                for (gen, new_elem) in successors_to_process.into_iter() {
                    let id;
                    match element_ids.entry(new_elem.clone()) {
                        // We've already seen `new_elem`.
                        approx_hashmap::Entry::Occupied(e) => {
                            id = *e.into_mut();

                            if id == GroupElementId::IDENTITY {
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
                            g.add_successor(next_unprocessed_id, gen)?;

                            // Enqueue a new task to compute the successors of
                            // `new_elem`.
                            let task = Arc::new(Task::new());
                            unprocessed_successors.push(Arc::clone(&task))?;
                            let generators_ref = &generators;
                            s.spawn(move |_| {
                                task.store(generators_ref.map_ref(|_id, gen| {
                                    (&new_elem * gen)
                                        .canonicalize()
                                        .unwrap_or_else(|| pga::Motor::ident(ndim))
                                }));
                            });

                            e.insert(id);
                        }
                    }
                    // Record the result of `new_elem * gen`.
                    g.set_successor(next_unprocessed_id, gen, id);
                }
                next_unprocessed_id.0 += 1;
            }
            // We've applyied every generator to every element, so we've
            // generated the whole group.

            Ok(())
        })?;

        let group = g.build()?;
        debug_assert_eq!(elements.len(), group.element_count());

        let nearest_neighbors =
            MotorNearestNeighborMap::new(&elements, elements.iter_keys().collect());

        Ok(Self {
            group,
            elements,
            nearest_neighbors,
        })
    }

    /// Returns the nearest element.
    pub fn nearest(&self, target: &pga::Motor) -> GroupElementId {
        match self.nearest_neighbors.nearest(target) {
            Some(&e) => e,
            None => GroupElementId::IDENTITY,
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

    #[test]
    fn test_cyclic_groups() {
        fn cyclic_group(n: Float) -> IsometryGroup {
            IsometryGroup::from_generators(&[pga::Motor::from_angle_in_normalized_plane(
                2,
                Vector::unit(0),
                Vector::unit(1),
                std::f64::consts::PI as Float * 2.0 / n,
            )])
            .unwrap()
        }

        assert_eq!(5, cyclic_group(5.0).element_count());

        assert_eq!(7, cyclic_group(7.0 / 2.0).element_count());
    }
}

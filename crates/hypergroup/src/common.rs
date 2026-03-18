use std::collections::VecDeque;

use hypermath::prelude::*;

use super::{AbbrGenSeq, GenSeq};

/// Generates a group or orbit by starting from an initial element and applying
/// generators recursively to find new objects. This function is the same as
/// [`orbit()`] except that it does not return a list of the elements and does
/// not pass an index into `apply_generator`.
///
/// `apply_generator` is called on every pair of an element and a generator. It
/// must return `Some(e * g)` if `e * g` has not yet been seen, or `None` if
/// `e * g` has already been seen.
pub(crate) fn orbit<E, G>(
    init: E,
    generators: &[G],
    mut apply_generator: impl FnMut(&E, &G) -> Option<E>,
) {
    let mut queue = VecDeque::new();
    queue.push_back(init);
    while let Some(elem) = queue.pop_front() {
        queue.extend(generators.iter().filter_map(|g| apply_generator(&elem, g)));
    }
}

/// Generates a group or orbit by starting from an initial element and applying
/// generators recursively to find new objects. Returns a list of all the
/// elements in the orbit, including `init`.
///
/// `apply_generator` is called on every pair of an element and a generator,
/// along with the index of the element in discovery order (where `init` has
/// index 0). It must return `Some(e * g)` if `e * g` has not yet been seen, or
/// `None` if `e * g` has already been seen.
pub(crate) fn orbit_collect<E, G>(
    init: E,
    generators: &[G],
    mut apply_generator: impl FnMut(usize, &E, &G) -> Option<E>,
) -> Vec<E> {
    let mut ret = vec![init];
    let mut next_unprocessed_index = 0;
    while next_unprocessed_index < ret.len() {
        for g in generators {
            let i = next_unprocessed_index;
            let elem = &ret[i];
            if let Some(new_elem) = apply_generator(i, elem, g) {
                ret.push(new_elem);
            }
        }
        next_unprocessed_index += 1;
    }
    ret
}

/// Returns the orbit of an object under the symmetry. Each generator is
/// specified along with its generator sequence.
pub fn orbit_geometric<T: Clone + ApproxHash + Ndim + TransformByMotor>(
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

    orbit_collect(
        (AbbrGenSeq::INIT, pga::Motor::ident(ndim), object),
        generators,
        |i, (_gen_seq, unprocessed_transform, unprocessed_object), (gen_seq_ids, generator)| {
            let mut new_object = generator.transform(unprocessed_object);
            if let approx_collections::hash_map::Entry::Vacant(e) =
                seen.entry_with_mut_key(&mut new_object)
            {
                e.insert(());
                let gen_seq = AbbrGenSeq {
                    generators: gen_seq_ids.clone(),
                    end: Some(i),
                };
                Some((gen_seq, generator * unprocessed_transform, new_object))
            } else {
                None
            }
        },
    )
}

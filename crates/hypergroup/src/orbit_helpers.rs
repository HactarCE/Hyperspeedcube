use std::collections::{HashSet, VecDeque};
use std::convert::Infallible;
use std::hash::Hash;

use hypermath::prelude::*;

use super::{AbbrGenSeq, GenSeq};

/// Error that can occur during orbit expansion.
#[derive(thiserror::Error, Debug, Copy, Clone)]
#[error("exceeded orbit limit of {0}")]
pub struct ExceededOrbitLimit(pub(crate) usize);

/// Generates a group or orbit by starting from an initial element and applying
/// generators recursively to find new objects. This function is the same as
/// [`orbit()`] except that it does not return a list of the elements and does
/// not pass an index into `apply_generator`.
///
/// `apply_generator` is called on every pair of an element and a generator. It
/// must return `Some(e * g)` if `e * g` has not yet been seen, or `None` if
/// `e * g` has already been seen.
pub fn orbit<E, G>(
    init: E,
    generators: &[G],
    mut apply_generator: impl FnMut(&E, &G) -> Option<E>,
) {
    try_orbit(init, generators, |elem, g| Ok(apply_generator(elem, g)))
        .unwrap_or_else(|e: Infallible| match e {})
}

/// Same as [`orbit()`], but returns early with an error if the number of
/// elements exceeds a limit.
pub fn orbit_with_limit<T, G>(
    limit: usize,
    init: T,
    generators: &[G],
    mut apply_generator: impl FnMut(&T, &G) -> Option<T>,
) -> Result<(), ExceededOrbitLimit> {
    let mut count = 0;
    try_orbit(init, generators, |elem, g| {
        let ret = apply_generator(elem, g);
        count += ret.is_some() as usize;
        if count < limit {
            Ok(ret)
        } else {
            Err(ExceededOrbitLimit(limit))
        }
    })
}

fn try_orbit<E, G, Err>(
    init: E,
    generators: &[G],
    mut apply_generator: impl FnMut(&E, &G) -> Result<Option<E>, Err>,
) -> Result<(), Err> {
    let mut queue = VecDeque::new();
    queue.push_back(init);
    while let Some(elem) = queue.pop_front() {
        for g in generators {
            if let Some(new_elem) = apply_generator(&elem, g)? {
                queue.push_back(new_elem);
            }
        }
    }
    Ok(())
}

/// Generates a group or orbit using `orbit`, collecting results in a
/// [`HashSet`].
pub fn orbit_hashable<E, G>(
    init: E,
    generators: &[G],
    mut apply_generator: impl FnMut(&E, &G) -> E,
) -> HashSet<E>
where
    E: Clone + Hash + Eq,
{
    let mut seen = HashSet::new();
    seen.insert(init.clone());
    orbit(init, generators, |e, g| {
        let new_elem = apply_generator(e, g);
        seen.insert(new_elem.clone()).then_some(new_elem)
    });
    seen
}

/// Generates a group or orbit by starting from an initial element and applying
/// generators recursively to find new objects. Returns a list of all the
/// elements in the orbit, including `init`.
///
/// `apply_generator` is called on every pair of an element and a generator,
/// along with the index of the element in discovery order (where `init` has
/// index 0). It must return `Some(e * g)` if `e * g` has not yet been seen, or
/// `None` if `e * g` has already been seen.
pub fn orbit_collect<E, G>(
    init: E,
    generators: &[G],
    mut apply_generator: impl FnMut(usize, &E, &G) -> Option<E>,
) -> Vec<E> {
    try_orbit_collect(init, generators, |i, elem, g| {
        Ok(apply_generator(i, elem, g))
    })
    .unwrap_or_else(|e: Infallible| match e {})
}

/// Same as [`orbit_collect()`], but returns early with an error if the number
/// of elements exceeds a limit.
pub fn orbit_collect_with_limit<E, G>(
    limit: usize,
    init: E,
    generators: &[G],
    mut apply_generator: impl FnMut(usize, &E, &G) -> Option<E>,
) -> Result<Vec<E>, ExceededOrbitLimit> {
    let mut count = 0;
    try_orbit_collect(init, generators, |i, elem, g| {
        let ret = apply_generator(i, elem, g);
        count += ret.is_some() as usize;
        if count < limit {
            Ok(ret)
        } else {
            Err(ExceededOrbitLimit(limit))
        }
    })
}

fn try_orbit_collect<E, G, Err>(
    init: E,
    generators: &[G],
    mut apply_generator: impl FnMut(usize, &E, &G) -> Result<Option<E>, Err>,
) -> Result<Vec<E>, Err> {
    let mut ret = vec![init];
    let mut next_unprocessed_index = 0;
    while next_unprocessed_index < ret.len() {
        for g in generators {
            let i = next_unprocessed_index;
            let elem = &ret[i];
            if let Some(new_elem) = apply_generator(i, elem, g)? {
                ret.push(new_elem);
            }
        }
        next_unprocessed_index += 1;
    }
    Ok(ret)
}

/// Returns the orbit of an object under the symmetry.
///
/// Returns an error if the number of elements exceeds the limit.
pub fn orbit_geometric<T: Clone + ApproxHash + Ndim + TransformByMotor>(
    limit: usize,
    generators: &[pga::Motor],
    mut object: T,
) -> Result<Vec<T>, ExceededOrbitLimit> {
    let mut seen = ApproxHashMap::new(APPROX);
    seen.entry_with_mut_key(&mut object).or_insert(());

    orbit_collect_with_limit(
        limit,
        object,
        generators,
        |_, unprocessed_object, generator| {
            let mut new_object = generator.transform(unprocessed_object);
            if let approx_collections::hash_map::Entry::Vacant(e) =
                seen.entry_with_mut_key(&mut new_object)
            {
                e.insert(());
                Some(new_object)
            } else {
                None
            }
        },
    )
}

/// Returns the orbit of an object under the symmetry. Each object in the orbit
/// is specified along with its generator sequence.
pub fn orbit_geometric_with_gen_seq<T: Clone + ApproxHash + Ndim + TransformByMotor>(
    limit: usize,
    generators: &[(GenSeq, pga::Motor)],
    mut object: T,
) -> Result<Vec<(AbbrGenSeq, pga::Motor, T)>, ExceededOrbitLimit> {
    let ndim = generators
        .iter()
        .map(|(_, m)| m.ndim())
        .max()
        .unwrap_or(1)
        .max(object.ndim());

    let mut seen = ApproxHashMap::new(APPROX);
    seen.entry_with_mut_key(&mut object).or_insert(());

    orbit_collect_with_limit(
        limit,
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

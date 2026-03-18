use std::collections::HashMap;

use itertools::Itertools;
use rand::{RngExt, SeedableRng, seq::IndexedRandom};

use super::*;

/// By running the product replacement algorithm on the generators for a
/// group before generating the group, we can get much shorter words for the
/// elements on average. This makes multiplying group elements and
/// transforming points much faster.
///
/// We can use the same number of generators, or add more generators. More
/// generators yields shorter words, but with diminishing returns. More
/// generators also requires more iterations of the product replacement
/// algorithm.
///
/// Empirically, most 3D and 4D groups (I tested H3 and H4) only need ~10
/// iterations to converge when not adding more generators. I100 takes
/// *many* more iterations (somewhere between 50 and 100), especially when
/// adding more generators. It may be worth running a few hundred iterations
/// on all groups to play it safe, especially considering product
/// replacement is so cheap to compute.
#[test]
fn product_replacement_word_len() -> eyre::Result<()> {
    let group = CoxeterMatrix::I(100)?.group()?;

    let generators = group.generators().to_vec();

    let mut permute_rng_1 = rand::rngs::StdRng::seed_from_u64(987654321);

    let mut generators = generators.clone();
    generators.resize(generators.len() * 3, GroupElementId::IDENTITY);
    for i in 0..100 {
        let unique_generators: PerGenerator<GroupElementId> = generators
            .iter()
            .copied()
            .filter(|&g| g != GroupElementId::IDENTITY)
            .sorted()
            .dedup()
            .collect();
        let mut new_to_old_element_map = PerGroupElement::from_iter([GroupElementId::IDENTITY]);
        let mut old_to_new_element_map: HashMap<_, _> =
            HashMap::from_iter([(GroupElementId::IDENTITY, GroupElementId::IDENTITY)]);
        let group = Group::from_compose_fn("", unique_generators.len(), |e, g| {
            let new_elem = group.compose(new_to_old_element_map[e], unique_generators[g]);
            Ok(*old_to_new_element_map
                .entry(new_elem)
                .or_insert_with(|| new_to_old_element_map.push(new_elem).unwrap()))
        })?;
        let avg_word_len = group
            .elements()
            .map(|e| group.factorization(e).count())
            .sum::<usize>() as f32
            / group.element_count() as f32;
        println!("{i} {avg_word_len}");

        let mut indices = (0..generators.len()).collect_vec();
        let i = indices.swap_remove(permute_rng_1.random_range(0..generators.len()));
        let j = *indices.choose(&mut permute_rng_1).unwrap();
        generators[i] = group.compose(generators[i], generators[j]);
    }
    Ok(())
}

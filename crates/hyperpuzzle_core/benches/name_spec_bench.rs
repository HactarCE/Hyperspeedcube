#![allow(missing_docs, unused_crate_dependencies)]

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use hyperpuzzle_core::{
    NameSpecBiMapBuilder, Piece, name_spec_matches_name, preferred_name_from_name_spec,
};
use itertools::Itertools;
use rand::seq::IndexedRandom;
use std::cell::RefCell;

fn criterion_benchmark(c: &mut Criterion) {
    let chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZ".chars().collect_vec();

    for size in [1000, 100, 10, 1] {
        let mut rng = rand::rng();

        let names = (0..size)
            .map(|_| {
                format!(
                    "{}{{{}{}{}}}",
                    chars.choose(&mut rng).unwrap(),
                    chars.choose(&mut rng).unwrap(),
                    chars.choose(&mut rng).unwrap(),
                    chars.choose(&mut rng).unwrap(),
                )
            })
            .collect_vec();

        let rng = RefCell::new(rng);

        let random_name =
            || preferred_name_from_name_spec(names.choose(&mut rng.borrow_mut()).unwrap());

        c.bench_with_input(
            BenchmarkId::new("name_spec_vec", size),
            &names,
            |b, input| {
                b.iter_batched(
                    &random_name,
                    |name_to_look_up| {
                        input
                            .iter()
                            .find(|n| name_spec_matches_name(n, &name_to_look_up))
                            .unwrap();
                    },
                    criterion::BatchSize::SmallInput,
                )
            },
        );

        let mut name_spec_map = NameSpecBiMapBuilder::new();
        let mut i = 0;
        for name in &names {
            if name_spec_map.set(Piece(i as _), Some(name.clone())).is_ok() {
                i += 1;
            }
        }
        println!("actually only {i} entries");
        let name_spec_map = name_spec_map.build(i).unwrap();

        c.bench_with_input(
            BenchmarkId::new("name_spec_map", size),
            &name_spec_map,
            |b, input| {
                b.iter_batched(
                    &random_name,
                    |name_to_look_up| input.id_from_name(&name_to_look_up).unwrap(),
                    criterion::BatchSize::SmallInput,
                )
            },
        );
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

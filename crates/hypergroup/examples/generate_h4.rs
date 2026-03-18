//! Example that generates the H4 Coxeter group using its representation as
//! isometries of 4D Euclidean space. This is also useful as an imprecise
//! benchmark.

#![allow(unused_crate_dependencies)]

use hypergroup::{CoxeterMatrix, IsometryGroup};

fn main() {
    let coxeter_matrix = CoxeterMatrix::H4();

    let order = 14400; // H4
    // let order = 51840; // E6

    let t = std::time::Instant::now();
    let g = coxeter_matrix.group().unwrap();
    println!("Generated abstract group in {:?}", t.elapsed());
    assert_eq!(order, g.element_count());

    let t = std::time::Instant::now();
    let g = coxeter_matrix.isometry_group().unwrap();
    println!("Generated isometry group {:?}", t.elapsed());
    assert_eq!(order, g.element_count());

    let t = std::time::Instant::now();
    let g = IsometryGroup::from_generators("H4", coxeter_matrix.generator_transforms().unwrap())
        .unwrap();
    println!("Generated isometry group from motors {:?}", t.elapsed());
    assert_eq!(order, g.element_count());

    let t = std::time::Instant::now();
    let g = coxeter_matrix.chiral_isometry_group().unwrap();
    println!(
        "Generated chiral isometry group from motors {:?}",
        t.elapsed()
    );
    assert_eq!(order / 2, g.element_count());
}

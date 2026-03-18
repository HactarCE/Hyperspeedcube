//! Example that generates the H4 Coxeter group using its representation as
//! isometries of 4D Euclidean space. This is also useful as an imprecise
//! benchmark.

#![allow(unused_crate_dependencies)]

use hypergroup::CoxeterMatrix;

fn main() {
    let coxeter_matrix = CoxeterMatrix::new_linear(&[5, 3, 3]).unwrap();

    let t = std::time::Instant::now();
    let g = coxeter_matrix.group().unwrap();
    println!("Generated abstract group in {:?}", t.elapsed());
    assert_eq!(14400, g.element_count());

    let t = std::time::Instant::now();
    let g = coxeter_matrix.isometry_group().unwrap();
    println!("Generated isometry group {:?}", t.elapsed());
    assert_eq!(14400, g.element_count());
}

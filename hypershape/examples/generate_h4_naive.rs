//! Example that generates the H4 Coxeter group using its representation as
//! isometries of 4D Euclidean space. This is also useful as an imprecise
//! benchmark.

use hypershape::group::{CoxeterGroup, Group};

fn main() {
    let before = std::time::Instant::now();
    let g = CoxeterGroup::new_linear(&[5, 3, 3])
        .unwrap()
        .group()
        .unwrap();
    let after = std::time::Instant::now();
    println!("{:?}", after - before);

    assert_eq!(14400, g.element_count());
}

//! Example that generates the H4 Coxeter group using the Todd-Coxeter
//! algorithm. This is also useful as an imprecise benchmark.

use hypershape::group::{FiniteCoxeterGroup, Group};

fn main() {
    let before = std::time::Instant::now();
    let g = FiniteCoxeterGroup::H4.group().unwrap(); // 120-cell
    let after = std::time::Instant::now();
    println!("{:?}", after - before);

    assert_eq!(g.element_count(), 14400);
}

//! Example that generates the H4 Coxeter group using its representation as
//! isometries of 4D Euclidean space. This is also useful as an imprecise
//! benchmark.

use hypershape::group::{Group, SchlafliSymbol};

fn main() {
    let before = std::time::Instant::now();
    let g = SchlafliSymbol::from_indices(vec![5, 3, 3]).group().unwrap();
    let after = std::time::Instant::now();
    println!("{:?}", after - before);

    assert_eq!(14400, g.element_count());
}

//! Multidimensional shape slicing and other geometric algorithms.

#![warn(
    rust_2018_idioms,
    missing_docs,
    clippy::cargo,
    clippy::if_then_some_else_none,
    clippy::manual_let_else,
    clippy::semicolon_if_nothing_returned,
    clippy::semicolon_inside_block,
    clippy::too_many_lines,
    clippy::undocumented_unsafe_blocks,
    clippy::unwrap_used
)]
#![allow(clippy::multiple_crate_versions)]

mod group;
mod schlafli;
mod space;

use std::collections::HashSet;

pub use group::*;
pub use schlafli::*;
pub use space::*;

/// Structs, traits, and constants.
pub mod prelude {
    pub use crate::group::*;
    pub use crate::schlafli::*;
    pub use crate::space::*;
}

#[cfg(test)]
mod tests;

#[test]
fn build_dodeca() {
    use hypermath::prelude::*;

    let mut s = Space::new(3);
    let g = SchlafliSymbol::from_indices(vec![5, 3]).group().unwrap();
    let mut shapes = ShapeSet::from_iter([s.whole_space()]);

    let mut seen = hypermath::collections::ApproxHashMap::new();
    for elem in g.elements() {
        for seed in vec![Vector::unit(2)] {
            // Ignore zero vectors
            if seed == vector![] {
                continue;
            }

            let v = g[elem].transform_vector(seed);
            if seen.insert(&v, ()).is_none() {
                let m = s
                    .add_manifold(Blade::ipns_plane(v, 1.0).ipns_to_opns(3))
                    .unwrap();
                shapes = s.carve(m).cut_set(shapes).unwrap();
            }
        }
    }

    let shape = shapes.iter().next().unwrap();
    let mut edges = HashSet::new();
    for face in s.boundary_of(shape) {
        println!("{}", s[s.manifold_of(face).id].blade);
        print!("  ");
        for edge in s.boundary_of(face) {
            print!(" {}", edge);
            edges.insert(edge.id);
        }
        println!();
    }
    println!();
    println!();
    for edge in edges {
        println!("{edge} = {}", s[s[edge].manifold].blade);
    }

    panic!()
}

//! Infinite Euclidean space in which flat polytopes can be constructed.

use std::collections::{hash_map, HashMap};
use std::fmt;
use std::ops::Index;

use eyre::{bail, ensure, eyre, OptionExt, Result};
use float_ord::FloatOrd;
use hypermath::collections::generic_vec::IndexOverflow;
use hypermath::collections::{ApproxHashMap, GenericVec};
use hypermath::prelude::*;
use itertools::Itertools;
use parking_lot::Mutex;
use tinyset::Set64;

mod cut;
mod cut_output;
mod map;
mod polytope;
mod space;

pub use cut::{Cut, CutParams, PolytopeFate};
pub use cut_output::PolytopeCutOutput;
pub use map::{SpaceMap, SpaceMapFor};
pub use polytope::{PolytopeData, PolytopeFlags};
pub use space::Space;

/// Set of vertices in a [`Space`].
pub type VertexSet = Set64<VertexId>;
/// Set of polytopes in a [`Space`].
pub type PolytopeSet = Set64<PolytopeId>;

hypermath::idx_struct! {
    /// ID for a memoized vertex in a [`Space`].
    pub struct VertexId(pub u32);
    /// ID for a memoized polytope in a [`Space`].
    pub struct PolytopeId(pub u32);
}

/// List containing a value per vertex.
pub type PerVertex<T> = GenericVec<VertexId, T>;
/// List containing a value per polytope.
pub type PerPolytope<T> = GenericVec<PolytopeId, T>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cube() {
        let mut space = Space::new(2);
        let root = space.add_primordial_cube(10.0).unwrap();
        println!("{}", space.dump_to_string(root));
        let result = space
            .cut_polytope(
                root,
                &mut Cut::carve(Hyperplane::from_pole(vector![1.0]).unwrap()),
            )
            .unwrap();
        match result {
            PolytopeCutOutput::Flush => println!("flush"),
            PolytopeCutOutput::NonFlush {
                inside,
                outside,
                intersection,
            } => {
                if let Some(p) = inside {
                    println!("inside = {}", space.dump_to_string(p));
                    println!();
                }
                if let Some(p) = outside {
                    println!("outside = {}", space.dump_to_string(p));
                    println!();
                }
                if let Some(p) = intersection {
                    println!("intersection = {}", space.dump_to_string(p));
                    println!();
                }
            }
        }
    }
}

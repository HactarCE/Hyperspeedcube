//! Infinite Euclidean space in which flat polytopes can be constructed.

use std::collections::{hash_map, HashMap};
use std::fmt;
use std::sync::{Arc, Weak};

use eyre::{bail, ensure, eyre, OptionExt, Result};
use float_ord::FloatOrd;
use hypermath::collections::generic_vec::IndexOverflow;
use hypermath::collections::{ApproxHashMap, GenericVec};
use hypermath::prelude::*;
use itertools::Itertools;
use parking_lot::Mutex;
use smallvec::{smallvec, SmallVec};
use tinyset::Set64;

mod cut;
mod cut_output;
mod elements;
mod map;
mod patchwork;
mod polytope_data;
mod primordial;
mod simplicial;
mod space;
mod spaceref;

pub use cut::{Cut, CutParams, PolytopeFate};
pub use cut_output::ElementCutOutput;
pub use elements::*;
pub use map::{SpaceMap, SpaceMapFor};
pub use patchwork::{Patch, Seam};
pub use polytope_data::PolytopeData;
pub use simplicial::{Simplex, SimplexBlob};
pub use space::Space;
pub use spaceref::SpaceRef;

hypermath::idx_struct! {
    /// ID for a memoized element of a polytope in a [`Space`].
    pub struct ElementId(pub u32);
    /// ID for a memoized top-level polytope in a [`Space`].
    pub struct PolytopeId(pub u32);
    /// ID for a memoized facet in a [`Space`].
    pub struct FacetId(pub u32);
    /// ID for a memoized face in a [`Space`].
    pub struct FaceId(pub u32);
    /// ID for a memoized edge in a [`Space`].
    pub struct EdgeId(pub u32);
    /// ID for a memoized vertex in a [`Space`].
    pub struct VertexId(pub u32);

    /// ID for a patch in a [`Space`].
    pub struct PatchId(pub u16);
    /// ID for a seam of a patch in a [`Space`].
    pub struct SeamId(pub u16);
}

/// List containing a value per polytope.
pub type PerElement<T> = GenericVec<ElementId, T>;
/// List containing a value per vertex.
pub type PerVertex<T> = GenericVec<VertexId, T>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cube() {
        let space = Space::new(2);
        let root = space.add_primordial_cube(10.0).unwrap();
        println!("{}", space.dump_to_string(root.as_element().id));
        let result = Cut::carve(&space, Hyperplane::from_pole(vector![1.0]).unwrap())
            .cut(root)
            .unwrap();
        match result {
            ElementCutOutput::Flush => println!("flush"),
            ElementCutOutput::NonFlush {
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

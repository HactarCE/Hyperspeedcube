//! Infinite Euclidean space in which flat polytopes can be constructed.

use std::collections::{HashMap, hash_map};
use std::fmt;

use eyre::{OptionExt, Result, bail, ensure, eyre};
use float_ord::FloatOrd;
use hypermath::prelude::*;
use hypuz_util::ti::{IndexOverflow, TiVec, flat_vec::FlatTiVec};
use itertools::Itertools;
use parking_lot::Mutex;
use smallvec::{SmallVec, smallvec};
use tinyset::Set64;

mod cut;
mod cut_output;
mod elements;
mod polytope_data;
mod simplicial;
mod space;
mod spaceref;

pub use cut::{Cut, CutParams, PolytopeFate};
pub use cut_output::ElementCutOutput;
pub use elements::*;
pub use polytope_data::PolytopeData;
pub use simplicial::{Simplex, SimplexBlob};
pub use space::Space;
pub use spaceref::SpaceRef;

hypuz_util::typed_index_struct! {
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

    /// ID for a memoized hyperplane in a [`Space`].
    pub struct HyperplaneId(pub u16);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cube() -> Result<()> {
        let mut space = Space::with_primordial_cube_radius(2, 10.0)?;
        let root: ElementId = space.primordial_cube().into();
        println!("{}", space.dump_to_string(root));
        let result =
            Cut::carve(Hyperplane::from_pole(vector![1.0]).unwrap()).cut(&mut space, root)?;
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
        Ok(())
    }
}

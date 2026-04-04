//! Infinite Euclidean space in which flat polytopes can be constructed.

use std::collections::{HashMap, VecDeque, hash_map};
use std::fmt;
use std::sync::atomic::AtomicU64;

use eyre::{OptionExt, Result, bail, ensure, eyre};
use float_ord::FloatOrd;
use hypermath::prelude::*;
use hypuz_util::ti::TiVec;
use hypuz_util::ti::flat_vec::FlatTiVec;
use itertools::Itertools;
use parking_lot::Mutex;
use smallvec::{SmallVec, smallvec};
use tinyset::Set64;

mod cut;
mod cut_output;
mod elements;
mod polytope_data;
mod portal_data;
mod simplicial;
mod space;
mod spaceref;

pub use cut::{Cut, CutParams, PolytopeFate};
pub use cut_output::ElementCutOutput;
pub use elements::*;
use polytope_data::{BoundaryPortals, PolytopeData};
pub use portal_data::PortalData;
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
    /// ID for a portal in a [`Space`].
    pub struct PortalId(pub u16);
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
            ElementCutOutput::Flush => panic!("flush"),
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
                assert!(inside.is_some());
                assert!(outside.is_none()); // carve
                assert!(intersection.is_some());
            }
        }
        Ok(())
    }

    #[test]
    fn test_portal_polygon() -> Result<()> {
        let mut space = Space::with_primordial_cube_radius(2, 5.0)?;
        let mut fundamental_region = space.primordial_cube().to_element_id(&space);

        // Schlafli symbol {6}
        let (sin_30, cos_30) = std::f64::consts::FRAC_PI_6.sin_cos(); // 30 degrees
        let mirror_planes = [
            Hyperplane::new(vector![1.0], 0.0).unwrap(),
            Hyperplane::new(vector![-cos_30, sin_30], 0.0).unwrap(),
        ];

        for m in &mirror_planes {
            fundamental_region = Cut::carve_portal(m.clone())
                .cut(&mut space, fundamental_region)?
                .outside()
                .unwrap();
            println!(
                "primordial cube fundamental region (partial) = {}",
                space.dump_to_string(fundamental_region),
            );
        }
        println!(
            "primordial cube fundamental region = {}",
            space.dump_to_string(fundamental_region),
        );

        // Cut a perfect hexagon
        let mut p = fundamental_region;
        let mut carve_pole = point![0.0, 1.0];
        for _ in 0..6 {
            for m in &mirror_planes {
                p = Cut::carve(Hyperplane::from_pole(carve_pole.as_vector()).unwrap())
                    .cut(&mut space, p)?
                    .inside()
                    .unwrap();
                carve_pole = m.reflect_point(&carve_pole);
            }
        }
        println!("hexagon fundamental region = {}", space.dump_to_string(p));
        assert_eq!(3, space.get(p).boundary().count()); // fundamental triangle of a hexagon

        let hexagon = space.unfold(p)?;
        println!("expanded hexagon = {}", space.dump_to_string(hexagon));
        assert_eq!(6, space.get(hexagon).boundary().count());

        // Cut an irregular 12-gon.
        let mut p = fundamental_region;
        let mut carve_pole = point![0.1, 1.0];
        for _ in 0..6 {
            for m in &mirror_planes {
                p = Cut::carve(Hyperplane::from_pole(carve_pole.as_vector()).unwrap())
                    .cut(&mut space, p)?
                    .inside()
                    .unwrap();
                carve_pole = m.reflect_point(&carve_pole);
            }
        }
        println!("12-gon fundamental region = {}", space.dump_to_string(p));
        assert_eq!(3, space.get(p).boundary().count()); // fundamental triangle of a 12-gon

        let dodecagon = space.unfold(p)?;
        println!("expanded 12-gon = {}", space.dump_to_string(dodecagon));
        assert_eq!(12, space.get(dodecagon).boundary().count());

        Ok(())
    }

    #[test]
    fn test_portal_cube() -> Result<()> {
        let mut space = Space::with_primordial_cube_radius(3, 5.0)?;

        // Schlafli symbol {4,3}
        let sqrt22 = 2.0_f64.recip().sqrt();
        let mirror_planes = [
            Hyperplane::new(vector![1.0], 0.0).unwrap(),
            Hyperplane::new(vector![-sqrt22, sqrt22], 0.0).unwrap(),
            Hyperplane::new(vector![0.0, -sqrt22, sqrt22], 0.0).unwrap(),
        ];
        let carve_planes = [Hyperplane::from_pole(vector![0.0, 0.0, 1.0]).unwrap()];
        let fundamental_region: ElementId =
            space.add_folded_shape(mirror_planes, carve_planes)?.into();

        println!(
            "cube fundamental region = {}",
            space.dump_to_string(fundamental_region)
        );
        assert_eq!(4, space.get(fundamental_region).boundary().count());

        let cube = space.unfold(fundamental_region)?;

        println!("cube = {}", space.dump_to_string(cube));
        assert_eq!(6, space.get(cube).boundary().count());

        Ok(())
    }
}

//! N-dimensional puzzle backend.
#![warn(clippy::if_then_some_else_none, missing_docs)]
#![deny(clippy::correctness)]

#[macro_use]
extern crate lazy_static;

#[macro_use]
pub mod collections;
#[macro_use]
pub mod math;
pub mod geometry;
// pub mod polytope;
pub mod puzzle;
pub mod util;

/// Numeric type used for layer masks.
pub type LayerMaskUint = u32;

/// Names for axes up to 8 dimensions.
pub const AXIS_NAMES: &str = "XYZWUVRS";

#[cfg(test)]
mod tests {
    use crate::{
        collections::VectorHashMap,
        geometry::{SchlafliSymbol, ShapeArena},
        math::{Matrix, VectorRef},
        puzzle::Mesh,
    };

    #[test]
    fn aaaaaaa() {
        const SCHLAFLI: &str = "3,2";
        let seeds = vec![vector![0.0, 1.0, 1.0]];

        let s = SchlafliSymbol::from_string(SCHLAFLI);
        let m = Matrix::from_cols(s.mirrors().iter().rev().map(|v| &v.0))
            .inverse()
            .unwrap_or(Matrix::EMPTY_IDENT) // TODO: isn't really right
            .transpose();
        let g = s.group().unwrap();

        let mut arena = ShapeArena::new_euclidean_cga(3);

        let mut f = 0;
        let mut seen = VectorHashMap::new();
        for elem in g.elements() {
            for seed in &seeds {
                let v = g[elem].transform_vector(seed);
                if seen.insert(v.clone(), ()).is_none() {
                    arena.carve_plane(&v, v.mag(), f).unwrap();
                    println!("{arena}");
                    f += 1;
                }
            }
        }

        println!("{arena}");

        Mesh::from_arena(&arena, false).unwrap();
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     macro_rules! test_puzzles_from_file {
//         ($(fn $test_name:ident: $file:literal),+ $(,)?) => {
//             $(
//                 #[test]
//                 fn $test_name() {
//                     test_puzzle_from_yaml(include_str!(concat!("../../puzzles/", $file, ".yaml")));
//                 }
//             )+
//         };
//     }

//     fn test_puzzle_from_yaml(s: &str) {
//         let spec: puzzle::spec::PuzzleSpec = serde_yaml::from_str(s).expect("error parsing spec");
//         dbg!(spec);
//         // spec.build(&mut vec![]).expect("error building puzzle");
//     }

//     test_puzzles_from_file! {
//         fn test_3x3x3: "3x3x3_goal",
//         fn test_3x3x3_min: "3x3x3_goal_min",
//         fn test_3x3x3_compact: "3x3x3_goal_compact",

//         // fn test_1x1x1: "1x1x1",
//         // fn test_1x1x1x1: "1x1x1x1",
//         // fn test_2x2x2: "2x2x2",
//         // fn test_2x2x2x2: "2x2x2x2",
//         // fn test_2x2x2x2x2: "2x2x2x2x2",
//         // fn test_2x2x2x4: "2x2x2x4",
//         // fn test_2x3x4: "2x3x4",
//         // fn test_2x3x4x5: "2x3x4x5",
//         // fn test_3x3: "3x3",
//         // fn test_3x3x3: "3x3x3",
//         // fn test_3x3x3x3: "3x3x3x3",
//         // fn test_3x3x3x5: "3x3x3x5",
//         // fn test_4x4x4x4: "4x4x4x4",
//         // fn test_5x5x5: "5x5x5",
//         // fn test_5x5x5x5: "5x5x5x5",
//         // fn test_10x10x10: "10x10x10",
//         // fn test_17x17x17: "17x17x17",
//         // fn test_dino: "Dino",
//         // fn test_fto: "FTO",
//         // fn test_half_rt: "Half_RT",
//         // fn test_half_vt: "Half_VT",
//         // fn test_helicopter: "Helicopter",
//         // fn test_hmt: "HMT",
//         // fn test_rhombic_dodecahedron: "Rhombic Dodecahedron",
//         // fn test_skewb: "Skewb",
//     }
// }

//! Common types and traits used for any puzzle.

#[macro_use]
mod common;

pub mod controller;
pub mod geometry;
pub mod rubiks_3d;
pub mod rubiks_4d;

pub use common::*;
pub use controller::*;
pub use geometry::*;
pub use rubiks_3d::Rubiks3D;
pub use rubiks_4d::Rubiks4D;

pub mod traits {
    pub use super::{PuzzleInfo, PuzzleState, PuzzleType};
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) fn test_twist_canonicalization(
        p: impl PuzzleType,
        mut twists_are_eq: impl FnMut(Twist, Twist) -> bool,
    ) {
        eprintln!("Testing twist canonicalization for {}", p.name());

        let all_twists = itertools::iproduct!(
            (0..p.twist_axes().len() as _).map(TwistAxis),
            (0..p.twist_directions().len() as _).map(TwistDirection),
            (1..(1 << p.layer_count())).map(LayerMask)
        )
        .map(|(axis, direction, layers)| Twist {
            axis,
            direction,
            layers,
        });

        // Every twist should be equivalent to its canonicalization.
        for twist in all_twists {
            let canonicalized = p.canonicalize_twist(twist);

            assert!(
                twists_are_eq(twist, canonicalized),
                "Twist does not match its canonicalization. \n\n\
                 Twist:\n{:?}\n\n\
                 Canonicalization:\n{:?}",
                twist,
                canonicalized,
            );
        }
    }
}

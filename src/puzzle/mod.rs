//! Common types and traits used for any puzzle.

#[macro_use]
mod common;

pub mod controller;
pub mod geometry;
pub mod notation;
pub mod rubiks_3d;
pub mod rubiks_4d;

pub use common::*;
pub use controller::*;
pub use geometry::*;
pub use notation::*;
pub use rubiks_3d::Rubiks3D;
pub use rubiks_4d::Rubiks4D;

pub mod traits {
    pub use super::{PuzzleInfo, PuzzleState, PuzzleType};
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    /// Test that every twist is equivalent to its canonicalization.
    pub(super) fn test_twist_canonicalization(
        p: &impl PuzzleType,
        mut twists_are_eq: impl FnMut(Twist, Twist) -> bool,
    ) {
        eprintln!("Testing twist canonicalization for {}", p.name());

        for twist in iter_all_twists(p) {
            let canonicalized = p.canonicalize_twist(twist);

            assert!(
                twists_are_eq(twist, canonicalized),
                "Twist for {} does not match its canonicalization. \n\n\
                 Twist:\n{:?}\n\n\
                 Canonicalization:\n{:?}",
                p.name(),
                twist,
                canonicalized,
            );
        }
    }

    /// Test that every canonical twist can be losslessly serialized/deserialized.
    pub(super) fn test_twist_serialization(p: &impl PuzzleType) {
        let mut seen = HashSet::new();
        test_twist_serialization_for_each(
            p,
            iter_all_twists(p)
                .map(|t| p.canonicalize_twist(t))
                .filter(|&t| seen.insert(t)),
        );
    }

    /// Test that one canonical twist with each possible layer mask can be
    /// losslessly serialized/deserialized.
    pub(super) fn test_layered_twist_serialization(p: &impl PuzzleType) {
        test_twist_serialization_for_each(
            p,
            iter_all_layer_masks(p)
                .map(|layers| Twist {
                    layers,
                    ..Default::default()
                })
                .map(|t| p.canonicalize_twist(t)),
        );
    }

    fn test_twist_serialization_for_each(
        p: &impl PuzzleType,
        twists: impl IntoIterator<Item = Twist>,
    ) {
        let notation = p.notation_scheme();

        for twist in twists {
            let serialized_twist = notation.twist_to_string(twist);
            let deserialized_twist = notation.parse_twist(&serialized_twist);
            assert_eq!(
                Ok(twist),
                deserialized_twist,
                "Error deserializing {:?} for {}",
                serialized_twist,
                p.name(),
            );
        }
    }

    fn iter_all_twists(p: &impl PuzzleType) -> impl Iterator<Item = Twist> {
        itertools::iproduct!(
            (0..p.twist_axes().len() as _).map(TwistAxis),
            (0..p.twist_directions().len() as _).map(TwistDirection),
            iter_all_layer_masks(p)
        )
        .map(|(axis, direction, layers)| Twist {
            axis,
            direction,
            layers,
        })
    }

    fn iter_all_layer_masks(p: &impl PuzzleType) -> impl Clone + Iterator<Item = LayerMask> {
        (1..(1 << p.layer_count())).map(LayerMask)
    }
}

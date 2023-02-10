use ahash::AHashMap;
use rand::seq::{IteratorRandom, SliceRandom};
use std::fmt;
use std::str::FromStr;

use super::*;
use crate::group::SymmetryGroup;
use crate::math::Rotor;

/// Puzzle twist set metadata.
#[derive(Debug)]
pub struct PuzzleTwists {
    /// Twist set name.
    pub name: String,

    /// Twist axes.
    pub axes: Vec<TwistAxisInfo>,
    /// Canonical ordering of twist axes.
    pub axis_order: Vec<TwistAxis>,
    /// List of twist axes with at least one twist transform and at least one cut.
    pub non_empty_axes: Vec<TwistAxis>,
    /// Twist axes listed by name.
    pub axes_by_name: AHashMap<String, TwistAxis>,

    /// Twist transforms.
    pub transforms: Vec<TwistTransformInfo>,

    /// Symmetry group of the set of twist axes. This is not necessarily the
    /// same as the symmetry group of the puzzle; for example, a cuboid could
    /// use cubic symmetry here.
    pub symmetry: SymmetryGroup,

    /// Notation system.
    pub notation: NotationScheme,
}
impl Default for PuzzleTwists {
    fn default() -> Self {
        Self {
            name: "none".to_string(),

            axes: vec![],
            axis_order: vec![],
            non_empty_axes: vec![], // TODO: do we need this?
            axes_by_name: AHashMap::new(),

            transforms: vec![],

            symmetry: SymmetryGroup::default(),

            notation: NotationScheme::default(),
        }
    }
}
impl_puzzle_info_trait!(for PuzzleTwists { fn info(TwistAxis) -> &TwistAxisInfo { .axes } });
impl_puzzle_info_trait!(for PuzzleTwists { fn info(TwistTransform) -> &TwistTransformInfo { .transforms } });
impl PuzzleTwists {
    /// Returns the twist axis with a particular name, if one exists.
    pub fn axis_from_name(&self, name: &str) -> Option<TwistAxis> {
        (0..self.axes.len() as u16)
            .map(TwistAxis)
            .find(|&twist_axis| self.info(twist_axis).name == name)
    }
    /// Returns the twist transform with a particular name, if one exists.
    pub fn transform_from_name(&self, name: &str) -> Option<TwistTransform> {
        (0..self.transforms.len() as u32)
            .map(TwistTransform)
            .find(|&twist_transform| self.info(twist_transform).name == name)
    }

    /// Returns the nearest orientation.
    pub fn nearest_orientation(&self, rot: &Rotor) -> Rotor {
        let (rotor, _generators) = self.symmetry.nearest_orientation(rot);
        rotor.clone()
    }
}

pub struct Controls {
    direction_names: Vec<String>,
    // directions: IndexMap<String>,
}
impl Controls {
    pub fn direction_names(&self) -> &[String] {
        &self.direction_names
    }
    pub fn to_twist(&self, axis: TwistAxis, direction: String) -> Twist {
        todo!()
    }
}

pub struct TwistInput {
    axis: TwistAxis,
    // direction: TwistInputDirection,
}

/// Twist that may be applied to a puzzle.
///
/// Comparison prioritizes layer mask, then twist transform ID.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Twist {
    /// Layers affected by the twist.
    pub layers: LayerMask,
    /// Twist transform.
    pub transform: TwistTransform,
}
impl fmt::Display for Twist {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{},{}", self.layers.0, self.transform.0)
    }
}
impl FromStr for Twist {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // IIFE to mimic `try_block`
        (|| {
            let mut segments = s.split(',');
            let layers = LayerMask(segments.next()?.parse().ok()?);
            let transform = TwistTransform(segments.next()?.parse().ok()?);
            if segments.next().is_some() {
                return None;
            }
            Some(Self { layers, transform })
        })()
        .ok_or(())
    }
}
impl Twist {
    /// Returns a random twist for a puzzle type, or `None` if the puzzle has no twists.
    pub fn from_rng(ty: &PuzzleType) -> Option<Self> {
        let mut rng = rand::thread_rng();
        let axis = *ty.twists.non_empty_axes.choose(&mut rng)?;
        let transform = *ty.info(axis).transforms.choose(&mut rng)?;
        let layers = LayerMask((1..ty.info(axis).all_layers().0).choose(&mut rng)?);
        Some(Self { layers, transform })
    }
}

use anyhow::{anyhow, Result};
use std::fmt;
use std::ops::{BitOr, Mul};

mod cga_euclidean;

use crate::math::*;
pub use cga_euclidean::EuclideanCgaManifold;

/// Closed, oriented manifold in an N-dimensional space.
///
/// Other requirements:
/// - A manifold used as a cut must divide the (N+1)-dimensional manifold
///   containing it into exactly two connected pieces.
/// - A 1D manifold must be topologically equivalent to a circle.
/// - A 0D manifold must be a point pair.
/// - A manifold greater than 0D must be connected.
/// - If manifold M1 is flush with manifold M2 and they have the same number of
///   dimensions, then M1 = ±M2.
///
/// I *think* the first two conditions are true for any closed and oriented
/// manifold, but I'm not sure so I listed them out just in case.
pub trait Manifold: fmt::Debug + fmt::Display + Clone {
    /// Point in the space.
    type Point: Clone + AbsDiffEq<Epsilon = f32>;

    /// Returns the number of dimensions of the manifold.
    ///
    /// A line has one dimension, a plane has two, etc.
    fn ndim(&self) -> Result<u8>;

    /// Constructs a point pair (represented by a 0D manifold).
    fn new_point_pair(a: &Self::Point, b: &Self::Point, space: &Self) -> Result<Self>;

    /// Returns the point pair represented by a 0D manifold.
    fn to_point_pair(&self) -> Result<[Self::Point; 2]>;

    /// Returns the orientation of three points relative to `self`, which is
    /// assumed to be a 1D manifold containing them.
    ///
    /// The result is undefined if `self` does not contain all three points.
    fn triple_orientation(&self, points: [&Self::Point; 3]) -> f32;

    /// Flips the manifold to its other orientation.
    fn flip(&self) -> Result<Self>;

    /// Returns the relative orienation between `self` and `other` if they are
    /// the same manifold, or `None` if they are distinct manifolds.
    fn relative_orientation(&self, other: &Self) -> Option<Sign>;

    /// Given the (N+1)-dimensional `space` containing `self` and N-dimensional
    /// `cut`, splits `self` by `cut`.
    fn split(&self, cut: &Self, space: &Self) -> Result<ManifoldSplit<Self>> {
        let ManifoldWhichSide {
            is_any_inside,
            is_any_outside,
        } = self.which_side(cut, space)?;

        match (is_any_inside, is_any_outside) {
            (false, false) => Ok(ManifoldSplit::Flush(self.relative_orientation(cut))),
            (true, false) => Ok(ManifoldSplit::Inside),
            (false, true) => Ok(ManifoldSplit::Outside),
            (true, true) => Ok(ManifoldSplit::Split {
                intersection_manifold: self
                    .intersect(cut, space)?
                    .ok_or_else(|| anyhow!("cannot split disconnected manifold"))?,
            }),
        }
    }

    /// Given the N-dimensional `space` containing (N-1)-dimensional `cut` and
    /// M-dimensional `self` where M<=N, returns the (M-1)-dimensional
    /// intersection of `self` and `cut`. If `self` and `cut` do not intersect
    /// or if any of the other preconditions are broken, this function may
    /// return `None` or garbage.
    fn intersect(&self, cut: &Self, space: &Self) -> Result<Option<Self>>;

    /// Given the N-dimensional `space` containing `self` and (N-1)-dimensional
    /// `cut`, returns whether `self` is at least partly contained in each half
    /// of `space` separated by `cut`. Which part of `space` is considered
    /// "inside" or "outside" depends on the orientations of `space` and `cut`.
    fn which_side(&self, cut: &Self, space: &Self) -> Result<ManifoldWhichSide>;

    /// Returns whether `p` is contained in each half of `space` separated by
    /// `self`.
    fn which_side_has_point(&self, p: &Self::Point, space: &Self) -> Result<ManifoldWhichSide>;
}

/// Result of splitting a manifold by another manifold.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ManifoldSplit<M> {
    /// The manifold is flush with the slice. The sign is positive if they have
    /// the same orientation, negative if they have opposite orientation, or
    /// `None` if they have differing numbers of dimensions.
    Flush(Option<Sign>),
    /// The manifold is entirely inside the slice.
    Inside,
    /// The manifold is entirely outside the slice.
    Outside,
    /// The manifold has parts on both sides of the slice.
    Split {
        /// (N-1)-dimensional intersection of the manifold with the slicing
        /// manifold. There is always an intersection; splitting a disconnected
        /// manifold is not allowed.
        ///
        /// `intersection_manifold` itself, however, may be disconnected -- for
        /// example, if it is a point pair.
        intersection_manifold: M,
    },
}
/// Result of splitting a manifold by another manifold without calculating the
/// intersection.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ManifoldWhichSide {
    /// The manifold is partially or entirely inside the slice.
    pub is_any_inside: bool,
    /// The manifold is partially or entirely outside the slice.
    pub is_any_outside: bool,
}
impl BitOr for ManifoldWhichSide {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        ManifoldWhichSide {
            is_any_inside: self.is_any_inside | rhs.is_any_inside,
            is_any_outside: self.is_any_outside | rhs.is_any_outside,
        }
    }
}
impl Mul<Sign> for ManifoldWhichSide {
    type Output = Self;

    fn mul(mut self, rhs: Sign) -> Self::Output {
        if rhs == Sign::Neg {
            std::mem::swap(&mut self.is_any_inside, &mut self.is_any_outside);
        }
        self
    }
}
impl ManifoldWhichSide {
    fn neither_side() -> Self {
        Self {
            is_any_inside: false,
            is_any_outside: false,
        }
    }
}

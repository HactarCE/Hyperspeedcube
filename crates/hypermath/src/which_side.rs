//! Enums for which side of one thing contains another thing.

use std::ops::{Mul, MulAssign, Neg};

use crate::Sign;

/// Position of a point relative to an oriented manifold that divides space.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum PointWhichSide {
    /// The point is on the manifold between inside and outside.
    On,
    /// The point is on the "inside" space relative to the manifold.
    Inside,
    /// The point is on the "outside" space relative to the manifold.
    Outside,
}
impl Neg for PointWhichSide {
    type Output = Self;

    fn neg(self) -> Self::Output {
        match self {
            PointWhichSide::Inside => PointWhichSide::Outside,
            PointWhichSide::Outside => PointWhichSide::Inside,
            other => other,
        }
    }
}
impl Mul<Sign> for PointWhichSide {
    type Output = Self;

    fn mul(self, rhs: Sign) -> Self::Output {
        match rhs {
            Sign::Pos => self,
            Sign::Neg => match self {
                Self::On => Self::On,
                Self::Inside => Self::Outside,
                Self::Outside => Self::Inside,
            },
        }
    }
}

/// Location of an object (such as a polytope) relative to the half-spaces on
/// either side of a cut.
///
/// A point cannot be `Split`, so instead we use [`PointWhichSide`] for that.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum WhichSide {
    /// The object is flush with the cut. *Every* point on the object is
    /// touching the cut.
    Flush,
    /// The object is inside the cut.m
    Inside,
    /// The object is entirely outside the cut.
    Outside,
    /// The object is split by the cut.
    Split,
}
impl Neg for WhichSide {
    type Output = Self;

    fn neg(self) -> Self::Output {
        match self {
            WhichSide::Inside => WhichSide::Outside,
            WhichSide::Outside => WhichSide::Inside,
            other => other,
        }
    }
}
crate::impl_mul_sign!(impl Mul<Sign> for WhichSide);
crate::impl_mulassign_sign!(impl MulAssign<Sign> for WhichSide);
impl WhichSide {
    /// Constructs a [`WhichSide`] from several representative point locations.
    pub fn from_points(points: impl IntoIterator<Item = PointWhichSide>) -> Self {
        let mut is_any_inside = false;
        let mut is_any_outside = false;
        for which_side in points {
            match which_side {
                PointWhichSide::On => (),
                PointWhichSide::Inside => is_any_inside = true,
                PointWhichSide::Outside => is_any_outside = true,
            }
        }
        match (is_any_inside, is_any_outside) {
            (true, true) => WhichSide::Split,
            (true, false) => WhichSide::Inside,
            (false, true) => WhichSide::Outside,
            (false, false) => WhichSide::Flush,
        }
    }
}

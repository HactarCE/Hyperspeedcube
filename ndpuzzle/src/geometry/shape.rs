use anyhow::Result;
use std::cmp::Ordering;
use std::fmt;
use std::ops::Neg;
use tinyset::{Fits64, Set64};

use super::Manifold;
use crate::math::Sign;

/// Subset of a connected manifold, defined as an intersection of half-spaces on
/// its surface.
#[derive(Debug, Clone)]
pub struct Shape<M> {
    /// N-dimensional surface of the shape, **which must be connected**.
    pub manifold: M,
    /// (N-1)-dimensional shapes which define the boundary of the N-dimensional
    /// shape.
    pub boundary: Set64<ShapeRef>,
}
impl<M: Manifold> Shape<M> {
    /// Constructs a shape that contains a whole manifold with no boundary.
    pub fn whole_space(manifold: M) -> Self {
        let boundary = Set64::new();
        Self { manifold, boundary }
    }

    /// Returns the number of dimensions of the shape.
    pub fn ndim(&self) -> Result<u8> {
        self.manifold.ndim()
    }
}

/// Oriented ID for a shape in a `ShapeArena`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ShapeRef {
    /// Unoriented shape ID.
    pub id: ShapeId,
    /// Orientation.
    pub sign: Sign,
}
impl fmt::Display for ShapeRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.sign, self.id)
    }
}
impl From<ShapeId> for ShapeRef {
    fn from(id: ShapeId) -> Self {
        ShapeRef {
            id,
            sign: Sign::Pos,
        }
    }
}
impl tinyset::Fits64 for ShapeRef {
    unsafe fn from_u64(x: u64) -> Self {
        Self {
            id: ShapeId::from_u64(x >> 1),
            sign: if x & 1 == 0 { Sign::Pos } else { Sign::Neg },
        }
    }

    fn to_u64(self) -> u64 {
        (self.id.to_u64() << 1) | self.sign as u64
    }
}
impl Neg for ShapeRef {
    type Output = Self;

    fn neg(mut self) -> Self::Output {
        self.sign = -self.sign;
        self
    }
}
impl PartialOrd for ShapeRef {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for ShapeRef {
    fn cmp(&self, other: &Self) -> Ordering {
        self.to_u64().cmp(&other.to_u64())
    }
}

/// Non-oriented ID for a shape in a `ShapeArena`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ShapeId(pub(super) u32);
impl fmt::Display for ShapeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}
impl tinyset::Fits64 for ShapeId {
    unsafe fn from_u64(x: u64) -> Self {
        Self(x as u32)
    }

    fn to_u64(self) -> u64 {
        self.0 as u64
    }
}

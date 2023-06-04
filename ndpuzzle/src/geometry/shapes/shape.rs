use anyhow::Result;
use std::cmp::Ordering;
use std::fmt;
use std::ops::Neg;
use tinyset::{Fits64, Set64};

use crate::geometry::Manifold;
use crate::math::Sign;

idx_struct! {
    /// Non-oriented ID for a shape in a `ShapeArena`.
    pub struct ShapeId(pub u32);
}

/// Metadata that can be attached to a shape.
pub type ShapeMetadata = u16;

/// Subset of a connected manifold, defined as an intersection of half-spaces on
/// its surface.
#[derive(Debug, Clone)]
pub struct Shape<M> {
    /// N-dimensional surface of the shape, **which must be connected**.
    pub manifold: M,
    /// (N-1)-dimensional shapes which define the boundary of the N-dimensional
    /// shape.
    pub boundary: Set64<ShapeRef>,

    /// Metadata associated with the positive side of the shape.
    pub positive_metadata: Option<ShapeMetadata>,
    /// Metadata associated with the negative side of the shape.
    pub negative_metadata: Option<ShapeMetadata>,
}
impl<M: Manifold> Shape<M> {
    /// Constructs a shape that contains a whole manifold with no boundary.
    pub fn whole_space(manifold: M) -> Self {
        Self::new(manifold, Set64::new())
    }

    /// Constructs a shape with a boundary.
    pub fn new(manifold: M, boundary: Set64<ShapeRef>) -> Self {
        Self {
            manifold,
            boundary,

            positive_metadata: None,
            negative_metadata: None,
        }
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

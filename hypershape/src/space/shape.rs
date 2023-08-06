use super::*;

/// Shape defined by a manifold and a boundary.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShapeData {
    /// Manifold of the shape.
    pub manifold: ManifoldId,
    /// Boundary of the shape, represented using a set of shapes with one less
    /// dimension.
    pub boundary: ShapeSet,
}
impl ShapeData {
    /// Constructs a shape that contains a whole manifold with no boundary.
    pub fn whole_manifold(manifold: ManifoldId) -> Self {
        Self::new(manifold, ShapeSet::new())
    }

    /// Constructs a shape with a boundary.
    pub fn new(manifold: ManifoldId, boundary: ShapeSet) -> Self {
        Self { manifold, boundary }
    }

    /// Constructs a point pair.
    pub fn point_pair(manifold: ManifoldId) -> Self {
        Self {
            manifold,
            boundary: ShapeSet::new(),
        }
    }
}

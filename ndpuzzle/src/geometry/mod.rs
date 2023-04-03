//! Geometric algorithms and data structures, particularly shape slicing.

use anyhow::Result;

use crate::math::*;

mod group;
mod schlafli;
mod shapes;

pub use group::*;
pub use schlafli::*;
pub use shapes::*;

/// Euclidean shape arena represented using conformal geometric algebra.
pub type CgaShapeArena = ShapeArena<EuclideanCgaManifold>;
/// Euclidean cut parameters represented using conformal geometric algebra.
pub type CgaCutParams = CutParams<EuclideanCgaManifold>;

impl CgaShapeArena {
    /// Constructs a new Euclidean shape arena represented using conformal
    /// geometric algebra.
    pub fn new_euclidean_cga(ndim: u8) -> Self {
        Self::new(EuclideanCgaManifold::whole_space(ndim))
    }

    /// Carves all root shapes in the arena, removing all shapes that are
    /// outside the sphere.
    ///
    /// - If `radius` is negative, the sphere is inside-out.
    pub fn carve_sphere(&mut self, center: impl VectorRef, radius: f32) -> Result<()> {
        self.cut(CutParams {
            cut: EuclideanCgaManifold::sphere(center, radius, self.space().ndim()?),
            remove_inside: false,
            remove_outside: true,
        })
    }
    /// Carves all root shapes in the arena, removing all shapes that are
    /// outside the plane.
    ///
    /// - If `distance` is positive, the origin is considered inside.
    /// - If `distance` is negative, the origin is considered outside.
    pub fn carve_plane(&mut self, normal: impl VectorRef, distance: f32) -> Result<()> {
        self.cut(CutParams {
            cut: EuclideanCgaManifold::plane(normal, distance, self.space().ndim()?),
            remove_inside: false,
            remove_outside: true,
        })
    }

    /// Slices all root shapes in the arena.
    ///
    /// - If `radius` is negative, the sphere is inside-out.
    pub fn slice_sphere(&mut self, center: impl VectorRef, radius: f32) -> Result<()> {
        self.cut(CutParams {
            cut: EuclideanCgaManifold::sphere(center, radius, self.space().ndim()?),
            remove_inside: false,
            remove_outside: false,
        })
    }
    /// Slices all root shapes in the arena.
    ///
    /// - If `distance` is positive, the origin is considered inside.
    /// - If `distance` is negative, the origin is considered outside.
    pub fn slice_plane(&mut self, normal: impl VectorRef, distance: f32) -> Result<()> {
        self.cut(CutParams {
            cut: EuclideanCgaManifold::plane(normal, distance, self.space().ndim()?),
            remove_inside: false,
            remove_outside: false,
        })
    }
}

#[cfg(test)]
mod tests;

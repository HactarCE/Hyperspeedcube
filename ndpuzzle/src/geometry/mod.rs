//! Geometric algorithms and data structures, particularly shape slicing.

use anyhow::Result;

use crate::math::*;

mod cutting;
mod group;
mod manifold;
mod schlafli;
mod shape;

pub use cutting::*;
pub use group::*;
pub use manifold::*;
pub use schlafli::*;
pub use shape::*;

/// Euclidean shape arena represented using conformal geometric algebra.
pub type CgaShapeArena = ShapeArena<EuclideanCgaManifold>;

impl CgaShapeArena {
    /// Constructs a new Euclidean shape arena represented using conformal
    /// geometric algebra.
    pub fn new_euclidean_cga(ndim: u8) -> Self {
        Self::new(EuclideanCgaManifold::whole_space(ndim))
    }

    /// Carves all roots in the arena, removing all shapes that are outside the
    /// sphere.
    ///
    /// - If `radius` is negative, the sphere is inside-out.
    pub fn carve_sphere(&mut self, center: impl VectorRef, radius: f32) -> Result<()> {
        self.cut(CutParams {
            cut: EuclideanCgaManifold::from_ipns(
                &cga::Blade::ipns_sphere(center, radius),
                self.space().ndim()?,
            )
            .unwrap(),
            remove_inside: false,
            remove_outside: true,
        })
    }
    /// Carves all roots in the arena, removing all shapes that are outside the
    /// plane.
    ///
    /// - If `distance` is positive, the origin is considered inside.
    /// - If `distance` is negative, the origin is considered outside.
    pub fn carve_plane(&mut self, normal: impl VectorRef, distance: f32) -> Result<()> {
        self.cut(CutParams {
            cut: EuclideanCgaManifold::from_ipns(
                &cga::Blade::ipns_plane(normal, distance),
                self.space().ndim()?,
            )
            .unwrap(),
            remove_inside: false,
            remove_outside: true,
        })
    }
}

#[cfg(test)]
mod tests;

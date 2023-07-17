//! N-dimensional puzzle generation and simulation.

use std::cmp::Ordering;

mod common;
// mod loader;
// pub mod jumbling;
// pub mod spec;

pub use common::*;

use crate::collections::GenericVec;
use crate::geometry::{Manifold, ShapeArena, ShapeId};
use crate::math::cga::Isometry;
use crate::math::{approx_cmp, Float, PointWhichSide, Vector, VectorRef};

/// Cut within a twist axis.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TwistCut {
    /// Planar cut perpendicular to the twist axis.
    Planar {
        /// Distance between the plane and the orgin.
        distance: Float,
    },
    /// Spherical cut centered along the twist axis.
    Spherical {
        /// Distance between the center of the sphere and the origin.
        center_distance: Float,
        /// Radius of the sphere.
        radius: Float,
    },
}
impl TwistCut {
    /// Compares a point to the twist cut.
    ///
    /// The point is given as two numbers: parallel distance along the axis from
    /// the origin, and squared perpendicular distance away from the axis line.
    fn which_side_has_point(self, parallel: Float, perpendicular_squared: Float) -> PointWhichSide {
        let distance = match self {
            TwistCut::Planar { distance } => parallel - distance,
            TwistCut::Spherical {
                center_distance,
                radius,
            } => {
                let parallel_distance = parallel - center_distance;
                let parallel_distance_squared = parallel_distance * parallel_distance;

                // Compute the distance from the point to the center of the sphere.
                let total_distance_squared = parallel_distance_squared + perpendicular_squared;

                let radius_squared = radius * radius;
                total_distance_squared - radius_squared
            }
        };

        match approx_cmp(&distance, &0.0) {
            Ordering::Less => PointWhichSide::Inside,
            Ordering::Equal => PointWhichSide::On,
            Ordering::Greater => PointWhichSide::Outside,
        }
    }

    fn is_flat(self) -> bool {
        match self {
            TwistCut::Planar { .. } => true,
            TwistCut::Spherical { .. } => false,
        }
    }

    fn manifold(self, normal: &Vector, space_ndim: u8) -> Manifold {
        match self {
            TwistCut::Planar { distance } => Manifold::new_hyperplane(normal, distance, space_ndim),
            TwistCut::Spherical {
                center_distance,
                radius,
            } => Manifold::new_hypersphere(normal * center_distance, radius, space_ndim),
        }
    }
}

pub struct TwistAxisGeometry {
    /// Vector pointing along the twist axis.
    normal: Vector,

    /// Cut manifolds along the twist axis that separate its layers.
    cuts: Vec<TwistCut>,

    /// Layer ID containing each possible region relative to the cuts.
    ///
    /// TODO: consider an optimized look-up table for simple sequential cut
    /// arrangements.
    layer_definitions: GenericVec<CutMask, LayerId>,
}
impl TwistAxisGeometry {
    fn layer_of_shape(
        &self,
        arena: &ShapeArena,
        shape: ShapeId,
        piece_transform_relative_to_axis: &Isometry,
    ) -> LayerMask {
        // let normal = piece_transform_relative_to_axis.reverse().transform_vector(vector![1.0    ]);

        // let cut_mask = CutMask(
        //     self.cuts.iter().enumerate().map(|(i,cut)| {
        //         let cut_manifold = cut.manifold(normal, arena.space().ndim()?);
        //         arena.shape_contains_point(shape, point)
        //         cut_manifold.whic
        //         (1 << i)
        //     }).sum()
        // )

        // TODO: make this work
        LayerMask(1)
    }
}

struct CutMask(u32);

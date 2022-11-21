//! Newell's algorithm for sorting convex polygons by depth, without polgyon
//! splitting.

use cgmath::*;
use itertools::Itertools;
use smallvec::SmallVec;

use super::{ClickTwists, Sticker, Twist};
use crate::util;

const W_NEAR_CLIPPING_DIVISOR: f32 = 0.1;
const Z_NEAR_CLIPPING_DIVISOR: f32 = 0.0;

use crate::math::{Matrix, VectorRef};

/// Parameters for constructing sticker geometry.
#[derive(Debug, Clone, PartialEq)]
pub struct StickerGeometryParams {
    /// `2 * (space between facet and edge of puzzle) / (puzzle diameter)`.
    /// Ranges from 0.0 to 1.0.
    pub facet_spacing: f32,
    /// `(space between stickers) / (sticker width)`. Ranges from 0.0 to 2.0.
    pub sticker_spacing: f32,

    /// `(sticker width + space between stickers) / (puzzle diameter)`. Ranges
    /// from 0.0 to 1.0.
    pub sticker_grid_scale: f32,
    /// `(facet width + space between stickers) / (puzzle diameter)`. Ranges
    /// from 0.0 to infinity.
    pub facet_scale: f32,
    /// `(sticker width) / (puzzle diameter)`. Ranges from 0.0 to 1.0.
    pub sticker_scale: f32,

    /// 4D FOV, in degrees.
    pub fov_4d: f32,
    /// 3D FOV, in degrees.
    pub fov_3d: f32,

    /// Factor of how much the W coordinate affects the XYZ coordinates. This is
    /// computed from the 4D FOV.
    pub w_factor_4d: f32,
    /// Factor of how much the Z coordinate affects the XY coordinates. This is
    /// computed from the 3D FOV.
    pub w_factor_3d: f32,

    /// Animated twist and animation progress.
    pub twist_animation: Option<(Twist, f32)>,
    /// View transformation matrix for the whole puzzle, after 4D projection.
    pub view_transform: Matrix,

    /// Whether to show frontfaces.
    pub show_frontfaces: bool,
    /// Whether to show backfaces.
    pub show_backfaces: bool,
    /// Whether to clip points behind the 4D camera.
    pub clip_4d: bool,
}
impl StickerGeometryParams {
    /// Returns the divisor for applying 4D perspective projection based on the
    /// W coordinate of a point.
    fn w_divisor(&self, w: f32) -> Option<f32> {
        let camera_w = self.facet_scale;

        // See `project_3d()` for an explanation of this formula. The only
        // differences here are that we assume the 4D FOV is positive and we
        // first normalize the W coordinate to have the camera at W=1.
        let divisor = 1.0 + (-w / camera_w) * self.w_factor_4d;

        // Clip geometry that is behind the 4D camera.
        if self.clip_4d && divisor <= W_NEAR_CLIPPING_DIVISOR {
            None
        } else {
            Some(divisor)
        }
    }

    /// Returns the divisor for applying 3D perspective projection based on the
    /// Z coordinate of a point.
    fn z_divisor(&self, z: f32) -> Option<f32> {
        // This formula gives us a divisor (which we would store in the W
        // coordinate, if we were doing this using the normal computer graphics
        // methods) that applies the desired FOV but keeps Z=1 fixed for
        // positive FOV, or Z=-1 fixed for negative FOV. This creates a really
        // awesome dolly zoom effect, where the puzzle stays roughly the same
        // size on the viewport even as the FOV changes.
        //
        // This Desmos graph shows how this divisor varies with respect to Z
        // (shown along the X axis) and the FOV (controlled by a slider):
        // https://www.desmos.com/calculator/ocztouh1h0
        let divisor = 1.0 + (self.fov_3d.signum() - z) * self.w_factor_3d;

        // Clip geometry that is behind the 3D camera.
        if divisor < Z_NEAR_CLIPPING_DIVISOR {
            None
        } else {
            Some(divisor)
        }

        // Wgpu wants a Z coordinate from 0 to 1, but because of the weird
        // rendering pipeline this program uses the GPU won't ever see this Z
        // coordinate. If you want to implement this dolly zoom effect yourself,
        // though, you'll probably need to consider that.
    }

    /// Applies the view transform and performs 4D perspective projection.
    pub fn project_4d(&self, point: impl VectorRef) -> Option<Point3<f32>> {
        let [x, y, z] = [0, 1, 2].map(|i| self.view_transform.col(i).dot(&point));
        let ret = cgmath::point3(x, y, z);
        if point.ndim() <= 3 {
            Some(ret)
        } else {
            let w = self.view_transform.col(3).dot(&point);
            // Divide by the W divisor.
            let w_divisor = self.w_divisor(w)?;
            let mult = w_divisor.recip();
            Some(ret * mult)
        }
    }

    /// Performs 3D perspective projection.
    pub fn project_3d(&self, point: Point3<f32>) -> Option<Point3<f32>> {
        // Divide by the Z divisor.
        let mut ret = point * self.z_divisor(point.z)?.recip();
        ret.z = point.z; // But keep the Z coordinate.
        Some(ret)
    }
}

/// Vertices for a sticker in 3D space.
pub struct StickerGeometry {
    /// Vertex positions, after 4D projection but before 3D projection.
    pub verts: Vec<Point3<f32>>,
    /// Indices for polygons.
    pub polygon_indices: Vec<Box<[u16]>>,
    /// Twists on left/right/middle mouse click per polygon.
    pub polygon_twists: Vec<ClickTwists>,
}
impl StickerGeometry {
    pub fn new_double_quad(
        verts: [Point3<f32>; 4],
        twists: ClickTwists,
        front_face: bool,
        back_face: bool,
    ) -> Self {
        let mut ret = Self {
            verts: verts.to_vec(),
            polygon_indices: vec![Box::new([0, 2, 3, 1]), Box::new([2, 0, 1, 3])],
            polygon_twists: vec![twists, twists.rev()],
        };
        if !back_face {
            ret.polygon_indices.pop();
            ret.polygon_twists.pop();
        }
        if !front_face {
            ret.polygon_indices.remove(0);
            ret.polygon_twists.remove(0);
        }
        ret
    }
    pub fn new_cube(verts: [Point3<f32>; 8], twists: [ClickTwists; 6]) -> Option<Self> {
        // Only show this sticker if the 3D volume is positive. (Cull it if its
        // 3D volume is negative.)
        Matrix3::from_cols(
            verts[4] - verts[0],
            verts[2] - verts[0],
            verts[1] - verts[0],
        )
        .determinant()
        .is_sign_positive()
        .then(|| Self {
            verts: verts.to_vec(),
            polygon_indices: vec![
                Box::new([0, 2, 3, 1]),
                Box::new([4, 5, 7, 6]),
                Box::new([0, 1, 5, 4]),
                Box::new([2, 6, 7, 3]),
                Box::new([0, 4, 6, 2]),
                Box::new([1, 3, 7, 5]),
            ],
            polygon_twists: twists.to_vec(),
        })
    }
}

#[derive(Debug)]
pub struct ProjectedStickerGeometry {
    pub sticker: Sticker,

    pub verts: Box<[Point3<f32>]>,
    pub min_bound: Point3<f32>,
    pub max_bound: Point3<f32>,

    pub front_polygons: Box<[Polygon]>,
    pub back_polygons: Box<[Polygon]>,
}
impl ProjectedStickerGeometry {
    pub fn twists_for_point(&self, point: Point2<f32>) -> Option<ClickTwists> {
        self.front_polygons
            .iter()
            .find(|polygon| polygon.contains_point(point))
            .map(|polygon| polygon.twists)
    }
}

#[derive(Debug, Clone)]
pub struct Polygon {
    pub verts: SmallVec<[Point3<f32>; 4]>,
    pub min_bound: Point3<f32>,
    pub max_bound: Point3<f32>,
    pub normal: Vector3<f32>,

    pub illumination: f32,

    pub twists: ClickTwists,
}
impl Polygon {
    /// Constructs a convex polygon from a list of coplanar vertices in
    /// counterclockwise order. The polygon must not be degenerate, and no three
    /// vertices may be colinear.
    pub fn new(verts: SmallVec<[Point3<f32>; 4]>, illumination: f32, twists: ClickTwists) -> Self {
        let mut min_bound = verts[0];
        let mut max_bound = verts[0];
        for v in &verts[1..] {
            if v.x < min_bound.x {
                min_bound.x = v.x;
            }
            if v.y < min_bound.y {
                min_bound.y = v.y;
            }
            if v.z < min_bound.z {
                min_bound.z = v.z;
            }

            if v.x > max_bound.x {
                max_bound.x = v.x;
            }
            if v.y > max_bound.y {
                max_bound.y = v.y;
            }
            if v.z > max_bound.z {
                max_bound.z = v.z;
            }
        }

        let normal = (verts[1] - verts[0]).cross(verts[2] - verts[0]).normalize();

        Self {
            verts,
            min_bound,
            max_bound,
            normal,

            illumination,

            twists,
        }
    }

    fn contains_point(&self, point: Point2<f32>) -> bool {
        self.min_bound.x <= point.x
            && self.min_bound.y <= point.y
            && point.x <= self.max_bound.x
            && point.y <= self.max_bound.y
            && self
                .verts
                .iter()
                .map(|v| cgmath::point2(v.x, v.y))
                .circular_tuple_windows()
                .all(|(a, b)| (b - a).perp_dot(point - a) <= 0.0)
    }
}

pub fn polygon_from_indices(
    verts: &[Point3<f32>],
    indices: &[u16],
    illumination: f32,
    twists: ClickTwists,
) -> Option<Polygon> {
    let verts: SmallVec<_> = indices.iter().map(|&i| verts[i as usize]).collect();
    let normal = polygon_normal_from_indices(&verts, &[0, 1, 2]);
    let (min_bound, max_bound) = util::min_and_max_bound(&verts)?;

    Some(Polygon {
        verts,
        min_bound,
        max_bound,
        normal,

        illumination,

        twists,
    })
}

pub fn polygon_normal_from_indices(verts: &[Point3<f32>], indices: &[u16]) -> Vector3<f32> {
    let a = verts[indices[0] as usize];
    let b = verts[indices[1] as usize];
    let c = verts[indices[2] as usize];
    (c - a).cross(b - a)
}

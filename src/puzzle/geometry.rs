//! Newell's algorithm for sorting convex polygons by depth, without polgyon
//! splitting.

use cgmath::*;
use smallvec::{smallvec, SmallVec};
use std::cmp::Ordering;

use super::Sticker;
use crate::preferences::ViewPreferences;
use crate::util::{self, f32_total_cmp, IterCyclicPairsExt};

const W_NEAR_CLIPPING_DIVISOR: f32 = 0.1;
const Z_NEAR_CLIPPING_DIVISOR: f32 = 0.0;

const EPSILON: f32 = 0.000001;

/// Parameters for constructing sticker geometry.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct StickerGeometryParams {
    /// Sticker spacing factor.
    pub sticker_spacing: f32,
    /// Face spacing factor.
    pub face_spacing: f32,

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

    /// Model transformation matrix for an individual sticker, before 4D
    /// projection.
    pub model_transform: Matrix4<f32>,
    /// View transformation matrix for the whole puzzle, after 4D projection.
    pub view_transform: Matrix3<f32>,

    /// Ambient lighting amount (0.0..=1.0).
    pub ambient_light: f32,
    /// Light vector (manitude of 0.0..=1.0).
    pub light_vector: Vector3<f32>,
}
impl StickerGeometryParams {
    /// Radius within which all puzzle geometry is expected to be.
    pub const CLIPPING_RADIUS: f32 = 2.0;

    /// Constructs sticker geometry parameters for a set of view preferences.
    pub fn new(view_prefs: &ViewPreferences) -> Self {
        // Compute the view and perspective transforms, which must be applied here
        // on the CPU so that we can do proper depth sorting.
        let view_transform = Matrix3::from_angle_x(Deg(view_prefs.pitch))
            * Matrix3::from_angle_y(Deg(view_prefs.yaw))
            / Self::CLIPPING_RADIUS;

        let ambient_light = 1.0 - view_prefs.light_intensity * 0.5; // TODO: make ambient light configurable
        let light_vector = Matrix3::from_angle_y(Deg(view_prefs.light_yaw))
        * Matrix3::from_angle_x(Deg(-view_prefs.light_pitch)) // pitch>0 means light comes from above
        * Vector3::unit_z() * view_prefs.light_intensity*0.5;

        Self {
            sticker_spacing: view_prefs.sticker_spacing,
            face_spacing: view_prefs.face_spacing,

            fov_4d: view_prefs.fov_4d,
            fov_3d: view_prefs.fov_3d,
            w_factor_4d: (view_prefs.fov_4d.to_radians() / 2.0).tan(),
            w_factor_3d: (view_prefs.fov_3d.to_radians() / 2.0).tan(),

            model_transform: Matrix4::identity(),
            view_transform,

            ambient_light,
            light_vector,
        }
    }

    /// Computes the sticker scale factor (0.0 to 1.0).
    pub fn sticker_scale(self) -> f32 {
        1.0 - self.sticker_spacing
    }
    /// Computes the sace scale factor (0.0 to 1.0).
    pub fn face_scale(self) -> f32 {
        (1.0 - self.face_spacing) * 3.0 / (2.0 + self.sticker_scale())
    }

    /// Projects a 4D point down to 3D.
    pub fn project_4d(self, point: Vector4<f32>) -> Option<Point3<f32>> {
        // See `project_3d()` for an explanation of this formula. The only
        // difference here is that we assume the 4D FOV is positive.
        let divisor = 1.0 + (1.0 - point.w) * self.w_factor_4d;

        // Clip geometry that is behind the 4D camera.
        if divisor < W_NEAR_CLIPPING_DIVISOR {
            return None;
        }

        Some(Point3::from_vec(point.truncate()) / divisor)
    }

    /// Projects a 3D point according to the perspective projection.
    pub fn project_3d(self, point: Point3<f32>) -> Option<Point3<f32>> {
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
        let divisor = 1.0 + (self.fov_3d.signum() - point.z) * self.w_factor_3d;

        // Clip geometry that is behind the 3D camera.
        if divisor < Z_NEAR_CLIPPING_DIVISOR {
            return None;
        }

        // Wgpu wants a Z coordinate from 0 to 1, but because of the weird
        // rendering pipeline this program uses the GPU won't ever see this Z
        // coordinate. If you want to implement this dolly zoom effect yourself,
        // though, you'll probably need to consider that.

        Some(point / divisor)
    }
}

/// Vertices for a sticker in 3D space.
pub struct StickerGeometry {
    /// Vertex positions, after 4D projection but before 3D projection.
    pub verts: Vec<Point3<f32>>,
    /// Indices for sticker faces.
    pub polygon_indices: Vec<Box<[u16]>>,
}
impl StickerGeometry {
    pub(super) fn new_double_quad(verts: [Point3<f32>; 4]) -> Self {
        Self {
            verts: verts.to_vec(),
            polygon_indices: vec![Box::new([0, 1, 3, 2]), Box::new([2, 3, 1, 0])],
        }
    }
    pub(super) fn new_cube(verts: [Point3<f32>; 8]) -> Self {
        Self {
            verts: verts.to_vec(),
            polygon_indices: vec![
                Box::new([0, 1, 3, 2]),
                Box::new([4, 6, 7, 5]),
                Box::new([0, 2, 6, 4]),
                Box::new([1, 5, 7, 3]),
                Box::new([0, 4, 5, 1]),
                Box::new([2, 3, 7, 6]),
            ],
        }
    }
}

#[derive(Debug)]
pub(crate) struct ProjectedStickerGeometry {
    pub sticker: Sticker,

    pub verts: Box<[Point3<f32>]>,
    pub min_bound: Point3<f32>,
    pub max_bound: Point3<f32>,

    pub front_polygons: Box<[Polygon]>,
    pub back_polygons: Box<[Polygon]>,
}
impl ProjectedStickerGeometry {
    pub(crate) fn contains_point(&self, point: Point2<f32>) -> bool {
        self.front_polygons
            .iter()
            .any(|polygon| polygon.contains_point(point))
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Polygon {
    pub verts: SmallVec<[Point3<f32>; 4]>,
    pub min_bound: Point3<f32>,
    pub max_bound: Point3<f32>,
    pub normal: Vector3<f32>,

    pub illumination: f32,
}
impl Polygon {
    /// Constructs a convex polygon from a list of coplanar vertices in
    /// counterclockwise order. The polygon must not be degenerate, and no three
    /// vertices may be colinear.
    pub fn new(verts: SmallVec<[Point3<f32>; 4]>, illumination: f32) -> Self {
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
                .cyclic_pairs()
                .all(|(a, b)| (b - a).perp_dot(point - a) <= 0.0)
    }
}

pub(crate) fn polygon_from_indices(
    verts: &[Point3<f32>],
    indices: &[u16],
    illumination: f32,
) -> Polygon {
    let verts: SmallVec<_> = indices.iter().map(|&i| verts[i as usize]).collect();
    let normal = polygon_normal_from_indices(&verts, &[0, 1, 2]);
    let (min_bound, max_bound) = util::min_and_max_bound(&verts);

    Polygon {
        verts,
        min_bound,
        max_bound,
        normal,

        illumination,
    }
}

pub(crate) fn polygon_normal_from_indices(verts: &[Point3<f32>], indices: &[u16]) -> Vector3<f32> {
    let a = verts[indices[0] as usize];
    let b = verts[indices[1] as usize];
    let c = verts[indices[2] as usize];
    (c - a).cross(b - a)
}

trait NewellObj: Sized {
    /// Aprroximates depth comparison. This method does not need to be accurate,
    /// but it should be fast.
    fn approx_depth_cmp(&self, other: &Self) -> Ordering;

    /// Returns `true` if `self` can be drawn behind `other`. Only returns
    /// `false` if `self` _must_ be drawn in front of `other`.
    fn can_be_drawn_behind(&self, other: &Self) -> bool;
}

/// Sort stickers by depth using to Newell's algorithm. Stickers are not split.
pub(crate) fn sort_by_depth(objs: &mut [ProjectedStickerGeometry]) {
    // First, approximate the correct order.
    objs.sort_by(NewellObj::approx_depth_cmp);

    // This algorithm is basically selection sort. At every iteration, all the
    // objects before `i` are guaranteed to be in their final order, and we
    // search for the object that we can place in the `i`th index.
    let mut i = 0;
    while i < objs.len() {
        // Keep track of how many times we swap objects; if we swap objects too
        // many times, we'll need to split one of them.
        let mut swaps = 0;

        // We want to advance `i`. In order to do that, we need to know that
        // `objs[i]` can be drawn behind all of `objs[(i+1)..]`.
        let mut j = i + 1;
        while j < objs.len() {
            if objs[i].can_be_drawn_behind(&objs[j]) {
                // All good! Advance `j`.
                j += 1;
            } else if i + swaps > j {
                // Hey wait, we've already tried swapping this polygon! There
                // must be a cycle, which can only be resolved by splitting one
                // of the objects. Dealing with split polygons is complicated,
                // so just give up and draw the polygons in the wrong order. :(
                break;
            } else {
                // Uh oh, `objs[j]` must be drawn behind `objs[i]`. Select
                // `objs[j]` to be drawn next by putting it at index `i` and
                // shifting everything else to the right.
                objs[i..=j].rotate_right(1);
                // Record that we swapped this polygon.
                swaps += 1;
                // Check all of `objs[(i+1)..]` again.
                j = i + 1;
            }
        }

        // Now we know that `objs[i]` can be drawn behind all of
        // `objs[(i+1)..]`, so we can advance `i`.
        i += 1;
    }

    // Everything is (hopefully) sorted now! Yay!
}

impl NewellObj for ProjectedStickerGeometry {
    fn approx_depth_cmp(&self, other: &Self) -> Ordering {
        f32_total_cmp(&self.min_bound.z, &other.min_bound.z)
    }

    fn can_be_drawn_behind(&self, other: &Self) -> bool {
        // 1. If `self` is completely behind `other`, then `self` can be drawn
        //    behind.
        if self.max_bound.z < other.min_bound.z {
            return true;
        }

        // 2. If the bounding boxes of `self` and `other` do not overlap on the
        //    screen, then `self` can be drawn behind.
        if self.max_bound.x < other.min_bound.x
            || self.max_bound.y < other.min_bound.y
            || other.max_bound.x < self.min_bound.x
            || other.max_bound.y < self.min_bound.y
        {
            return true;
        }

        // 3. If there is some back face of `other` such that every vertex of
        //    `self` is behind that plane, then `self` can be drawn behind.
        if other
            .back_polygons
            .iter()
            .any(|p| self.verts.iter().all(|&v| p.height_of_point(v) >= -EPSILON))
        {
            return true;
        }

        // 4. If there is some front face of `self` such that every vertex of
        //    `other` is in front of that plane of `self`, then `self` can be
        //    drawn behind.
        if self.front_polygons.iter().any(|p| {
            other
                .verts
                .iter()
                .all(|&v| p.height_of_point(v) >= -EPSILON)
        }) {
            return true;
        }

        // 5. If each front face of `self` can be drawn behind each front face
        //    of `other`, then `self` can be drawn behind.
        self.front_polygons.iter().all(|p| {
            other
                .front_polygons
                .iter()
                .all(|q| p.can_be_drawn_behind(q))
        })
    }
}

impl Polygon {
    /// Returns the height of a point above or below the plane of `self`.
    fn height_of_point(&self, point: Point3<f32>) -> f32 {
        (point - self.verts[0]).dot(self.normal)
    }

    /// Returns the screen-space intersection of `self` and `other`.
    fn xy_intersection(&self, other: &Self) -> Option<Self> {
        let mut verts = self.verts.clone();
        for (o1, o2) in other.edges() {
            let other_line = (point2(o1.x, o1.y), point2(o2.x, o2.y));

            let mut new_verts = smallvec![];

            for self_edge in verts
                .iter()
                .map(move |&v| PointRelativeToLine::new(v, other_line))
                .cyclic_pairs()
                .map(|(a, b)| LineSegmentRelativeToLine { a, b })
            {
                if self_edge.a.h >= -EPSILON {
                    new_verts.push(self_edge.a.p);
                }
                if let Some(intermediate) = self_edge.intersection() {
                    new_verts.push(intermediate);
                }
            }

            verts = new_verts;
        }

        (verts.len() >= 3).then(|| Polygon::new(verts, self.illumination))
    }

    fn edges<'a>(&'a self) -> impl 'a + Iterator<Item = (Point3<f32>, Point3<f32>)> {
        let v1s = self.verts.iter().copied();
        let v2s = self.verts.iter().copied().cycle().skip(1);
        v1s.zip(v2s)
    }
}
impl NewellObj for Polygon {
    fn approx_depth_cmp(&self, other: &Self) -> Ordering {
        f32_total_cmp(&self.min_bound.z, &other.min_bound.z)
    }

    fn can_be_drawn_behind(&self, other: &Self) -> bool {
        // 1. If `self` is completely behind `other`, then `self` can be drawn
        //    behind.
        if self.max_bound.z < other.min_bound.z {
            return true;
        }

        // 2. If the bounding boxes of `self` and `other` do not overlap on the
        //    screen, then `self` can be drawn behind.
        if self.max_bound.x < other.min_bound.x
            || self.max_bound.y < other.min_bound.y
            || other.max_bound.x < self.min_bound.x
            || other.max_bound.y < self.min_bound.y
        {
            return true;
        }

        // 3. If every vertex of `self` is behind the plane of `other`, then
        //    `self` can be drawn behind.
        if self
            .verts
            .iter()
            .all(|&v| other.height_of_point(v) <= EPSILON)
        {
            return true;
        }

        // 4. If every vertex of `other` is in front of the plane of `self`,
        //    then `self` can be drawn behind.
        if other
            .verts
            .iter()
            .all(|&v| self.height_of_point(v) >= -EPSILON)
        {
            return true;
        }

        // 5. If `self` and `other` do not overlap on the screen, then `self`
        //    can be drawn behind.
        if let Some(intersection) = self.xy_intersection(other) {
            // 6. If `self` is always behind the plane of `other` whenever they
            //    intersect, then `self` can be drawn behind.
            if intersection
                .verts
                .iter()
                .all(|&v| other.height_of_point(v) <= EPSILON)
            {
                return true;
            } else {
                // If we've reached this point, then there is some part of
                // `self` that must be drawn in front of `other`.
                return false;
            }
        } else {
            return true;
        }
    }
}

struct LineSegmentRelativeToLine {
    a: PointRelativeToLine,
    b: PointRelativeToLine,
}
impl LineSegmentRelativeToLine {
    /// Returns the intersection of the line segment with the line.
    fn intersection(self) -> Option<Point3<f32>> {
        (self.a.h.signum() != self.b.h.signum()).then(|| {
            let delta = self.b.p - self.a.p;
            let ratio = self.a.h / (self.a.h - self.b.h);
            self.a.p + delta * ratio
        })
    }
}

/// Point that is some distance above or below a line.
#[derive(Debug, Copy, Clone)]
struct PointRelativeToLine {
    /// Point.
    p: Point3<f32>,
    /// Distance ("height") of point, relative to the line.
    h: f32,
}
impl PointRelativeToLine {
    fn new(p: Point3<f32>, line: (Point2<f32>, Point2<f32>)) -> Self {
        Self {
            p,
            h: (cgmath::point2(p.x, p.y) - line.0).perp_dot(line.1 - line.0),
        }
    }
}

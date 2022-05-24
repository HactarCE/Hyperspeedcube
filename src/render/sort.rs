//! Newell's algorithm for sorting convex polygons by depth, without polgyon
//! splitting.

use cgmath::*;
use smallvec::{smallvec, SmallVec};
use std::cmp::Ordering;

use super::{f32_total_cmp, IterCyclicPairsExt, Polygon, ProjectedStickerGeometry};

const EPSILON: f32 = 0.000001;

pub(super) trait NewellObj: Sized {
    /// Aprroximates depth comparison. This method does not need to be accurate,
    /// but it should be fast.
    fn approx_depth_cmp(&self, other: &Self) -> Ordering;

    /// Returns `true` if `self` can be drawn behind `other`. Only returns
    /// `false` if `self` _must_ be drawn in front of `other`.
    fn can_be_drawn_behind(&self, other: &Self) -> bool;
}

/// Sort stickers by depth according to Newell's algorithm.
pub(super) fn sort_by_depth(objs: &mut [ProjectedStickerGeometry]) {
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

        (verts.len() >= 3).then(|| Polygon::new(verts, self.color))
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
impl Polygon {
    /// Constructs a convex polygon from a list of coplanar vertices in
    /// counterclockwise order. The polygon must not be degenerate, and no three
    /// vertices may be colinear.
    pub fn new(verts: SmallVec<[Point3<f32>; 4]>, color: [f32; 4]) -> Self {
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

            color,
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

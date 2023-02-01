use cgmath::{prelude::*, Point2};

use crate::math::{approx_eq, lerp};

/// Returns the intersection of two lines in 2D Euclidean space.
pub fn intersect_line_line(
    [a1, b1]: [Point2<f64>; 2],
    [a2, b2]: [Point2<f64>; 2],
) -> Option<Point2<f64>> {
    // This is my own algorithm because I have one that I like better than the
    // one Roice uses. https://www.desmos.com/calculator/r3xmn5uegx
    // - HactarCE

    let v2 = b2 - a2;

    // Compute signed distance ("height") from `line2` to `a1` and `b1`.
    let ah = v2.perp_dot(a1 - a2);
    let bh = v2.perp_dot(b1 - a2);

    // Without loss of generality, suppose `a1` is above `line2` (so `ah` is
    // positive) and `b1` is below (so `bh` is negative). Then this subtraction
    // actually gives a sum of the absolute values.
    let sum = ah - bh;

    if approx_eq(sum, 0.0) {
        // The lines are parallel.
        return None;
    }

    // This gives the intersection point along `line1`.
    let t = ah / sum;

    Some(lerp(a1, b1, t))
}

/// Returns the distance from a point to a line in 2D Euclidean space.
pub fn distance_point_line(p: Point2<f64>, line: [Point2<f64>; 2]) -> f64 {
    // When the line is actually just a point, Roice's code returns NaN but we
    // return the distance from between `p` and the only point on the "line."
    p.distance(project_onto_line(p, line))
}

/// Projects a point onto a line in 2D Euclidean space.
pub fn project_onto_line(p: Point2<f64>, [a, b]: [Point2<f64>; 2]) -> Point2<f64> {
    if approx_eq(a, b) {
        // The line is actually just a point. Return the only point on the
        // "line."
        return a;
    }

    a + (p - a).project_on(b - a)
}

use cgmath::{prelude::*, Basis2, Deg, Point2};

use crate::geometry::euclidean_2d;
use crate::math::approx_cmp;

/// Generalized circle (lines are a limiting case).
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Circle {
    /// Circle with finite radius.
    Circle { center: Point2<f64>, radius: f64 },
    /// Circle with infinite radius.
    Line([Point2<f64>; 2]),
}
impl Circle {
    /// Constructs a circle going through 3 points.
    pub fn from_3_points(p1: Point2<f64>, p2: Point2<f64>, p3: Point2<f64>) -> Self {
        // Check for any infinite points, in which case this is a line.
        if !p1.is_finite() {
            return Self::from_2_points(p2, p3);
        }
        if !p2.is_finite() {
            return Self::from_2_points(p1, p3);
        }
        if !p3.is_finite() {
            return Self::from_2_points(p1, p2);
        }

        // Some links:
        // http://mathforum.org/library/drmath/view/54323.html
        // http://delphiforfun.org/Programs/Math_Topics/circle_from_3_points.htm
        // There is lots of info out there about solving via equations, but as
        // with other code in this project (R3.Core), I wanted to use
        // geometrical constructions.
        // - Roice Nelson

        // Midpoints.
        let m1 = Point2::midpoint(p1, p2);
        let m2 = Point2::midpoint(p1, p3);

        // Perpendicular vectors.
        let rot90 = Basis2::from_angle(Deg(90.0));
        let b1 = rot90.rotate_vector(p2 - p1).normalize();
        let b2 = rot90.rotate_vector(p3 - p1).normalize();

        // Intersect the perpendicular bisectors to find the center of the
        // circle.
        let Some(center) = euclidean_2d::intersect_line_line([m1, m1 + b1], [m2, m2 + b2]) else {
            // The points are colinear, so this is a line.
            return Self::from_2_points(p1, p2);
        };

        let radius = (p1 - center).magnitude();

        debug_assert_approx_eq!(radius, (p2 - center).magnitude());
        debug_assert_approx_eq!(radius, (p3 - center).magnitude());

        Self::Circle { center, radius }
    }
    /// Constructs a circle with infinite radius going through 2 points.
    pub fn from_2_points(p1: Point2<f64>, p2: Point2<f64>) -> Self {
        // Normalize the line so that P1 is the closest point to the origin and
        // the direction vector has unit length. This lets us compare lines.
        let v = (p2 - p1).normalize();

        let p = euclidean_2d::project_onto_line(Point2::origin(), [p1, p2]);
        let mut line = [p, p + v];

        // Canonicalize the order of the points so that we can compare lines.
        match approx_cmp(p1.x, p2.x) {
            std::cmp::Ordering::Less => (),
            std::cmp::Ordering::Greater => line.reverse(),
            std::cmp::Ordering::Equal => match approx_cmp(p1.y, p2.y) {
                std::cmp::Ordering::Less => (),
                std::cmp::Ordering::Greater => line.reverse(),
                std::cmp::Ordering::Equal => (), // should not happen
            },
        }

        Self::Line(line)
    }

    /// Returns whether the circle has infinite radius and so is actually a
    /// line.
    pub fn is_line(self) -> bool {
        matches!(self, Self::Line { .. })
    }
}

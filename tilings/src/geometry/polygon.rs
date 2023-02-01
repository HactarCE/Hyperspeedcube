use cgmath::{prelude::*, Basis2, Deg, Point2, Rad, Vector2};
use itertools::Itertools;

use super::{Circle, Geometry, Schlafli};

/// Clockwise vs. counterclockwise.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ArcDirection {
    Clockwise,
    Counterclockwise,
}
impl ArcDirection {
    pub fn opposite(self) -> Self {
        match self {
            Self::Clockwise => Self::Counterclockwise,
            Self::Counterclockwise => Self::Clockwise,
        }
    }
}

/// Arc of a circle in 2D space.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Arc {
    /// Initial endpoint of the arc.
    pub start: Point2<f64>,
    /// Final endpoint of the arc.
    pub end: Point2<f64>,
    /// Center of the circle.
    pub center: Point2<f64>,
    /// Direction around the circle from `start` to `end`.
    pub direction: ArcDirection,
}
impl Arc {
    /// Returns the radius of the circle.
    pub fn radius(self) -> f64 {
        (self.start - self.center).magnitude()
    }

    /// Returns the angle swept by the arc.
    pub fn angle(self) -> Rad<f64> {
        let v1 = self.start - self.center;
        let v2 = self.end - self.center;
        let angle_ccw = v1.angle(v2).normalize();
        match self.direction {
            ArcDirection::Clockwise => Rad::full_turn() - angle_ccw,
            ArcDirection::Counterclockwise => angle_ccw,
        }
    }

    /// Returns the arc length.
    pub fn length(self) -> f64 {
        self.radius() * Rad::from(self.angle()).0
    }

    /// Returns the midpoint along the arc.
    pub fn midpoint(self) -> Point2<f64> {
        let half_angle = self.angle() / 2.0;
        let start_vector = self.start - self.center;
        let mid_vector = Basis2::from_angle(half_angle).rotate_vector(start_vector);
        self.end + mid_vector
    }

    /// Returns the reverse segment, which has the endpoints swapped and the
    /// opposite direction.
    #[must_use]
    pub fn reverse(self) -> Self {
        Self {
            start: self.end,
            end: self.start,
            center: self.center,
            direction: self.direction.opposite(),
        }
    }
}

/// Segment of a polygon.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Segment {
    Line([Point2<f64>; 2]),
    Arc(Arc),
}
impl Segment {
    /// Constructs a segment of a polygon that is a straight line.
    pub fn line([a, b]: [Point2<f64>; 2]) -> Self {
        Self::Line([a, b])
    }
    /// Constructs a segment of a polygon that is a curved arc.
    pub fn arc(start: Point2<f64>, mid: Point2<f64>, end: Point2<f64>) -> Self {
        match Circle::from_3_points(start, mid, end) {
            Circle::Line(line) => return Self::line(line),
            Circle::Circle { center, .. } => {
                // This is a simpler algorithm than the one that Roice uses.
                // https://www.desmos.com/calculator/9jopllzcn4
                let direction = if (start - mid).perp_dot(end - mid) < 0.0 {
                    ArcDirection::Counterclockwise
                } else {
                    ArcDirection::Clockwise
                };

                Self::Arc(Arc {
                    start,
                    end,
                    center,
                    direction,
                })
            }
        }
    }

    /// Returns the length.
    pub fn length(self) -> f64 {
        match self {
            Segment::Line([a, b]) => (b - a).magnitude(),
            Segment::Arc(arc) => arc.length(),
        }
    }
    /// Returns the midpoint along the segment.
    pub fn midpoint(self) -> Point2<f64> {
        match self {
            Segment::Line([a, b]) => Point2::midpoint(a, b),
            Segment::Arc(arc) => arc.midpoint(),
        }
    }

    /// Returns the reverse segment, which has its endpoints swapped.
    #[must_use]
    pub fn reverse(self) -> Self {
        match self {
            Segment::Line([a, b]) => Segment::Line([b, a]),
            Segment::Arc(arc) => Segment::Arc(arc.reverse()),
        }
    }
}

/// Projection of a polygon into 2D Euclidean space.
#[derive(Debug, Clone)]
pub struct Polygon {
    pub segments: Vec<Segment>,
    pub center: Point2<f64>,
}
impl Polygon {
    /// Constructs a polygon from a list of segments.
    pub fn from_segments(segments: Vec<Segment>) -> Self {
        let center = Self::approximate_centroid(&segments);

        Self { segments, center }
    }

    /// Constructs a polygon from a list of points.
    pub fn from_points(points: &[Point2<f64>]) -> Self {
        let segments = points
            .iter()
            .circular_tuple_windows()
            .map(|(&p1, &p2)| Segment::Line([p1, p2]))
            .collect();
        Self::from_segments(segments)
    }

    /// Constructs the Euclidean projection of a regular polygon from a Schlafli
    /// symbol.
    pub fn new_regular(schlafli: Schlafli) -> Self {
        let angle = Rad::full_turn() / schlafli.p as f64;

        let initial_point = cgmath::point2(schlafli.normalized_circumradius, 0.0);

        let points = (0..schlafli.p)
            .map(|i| Basis2::from_angle(angle * i as f64).rotate_point(initial_point));

        let segments = points
            .circular_tuple_windows()
            .map(|(start, end)| match schlafli.geometry {
                Geometry::Euclidean => Segment::Line([start, end]),
                Geometry::Spherical | Geometry::Hyperbolic => {
                    let center = if schlafli.p == 2 {
                        // Our magic formula below breaks down for digons.
                        // - Roice Nelson
                        let factor = Deg(30.0).tan();
                        let y = -schlafli.normalized_circumradius * start.x.signum() * factor;
                        cgmath::point2(0.0, y)
                    } else {
                        // Our segments are arcs in Non-Euclidean geometries.
                        // Magically, the same formula turned out to work for
                        // both. (Maybe this is because the Poincare Disc model
                        // of the hyperbolic plane is stereographic projection
                        // as well).
                        // - Roice Nelson

                        // If we ever want to support q=infinity, we'll need to
                        // make `piq` = 0 in that case.
                        let piq = Deg(180.0) / schlafli.q as f64;
                        let t1 = Deg(180.0) / schlafli.p as f64;
                        let t2 = Deg(90.0) - piq - t1;
                        let factor = (t1.tan() / t2.tan() + 1.0) / 2.0;
                        Point2::from_vec((start.to_vec() + end.to_vec()) * factor)
                    };
                    let direction = match schlafli.geometry {
                        Geometry::Spherical => ArcDirection::Counterclockwise,
                        Geometry::Euclidean => unreachable!(),
                        Geometry::Hyperbolic => ArcDirection::Clockwise,
                    };

                    Segment::Arc(Arc {
                        start,
                        end,
                        center,
                        direction,
                    })
                }
            })
            .collect();

        Self::from_segments(segments)
    }

    /// Returns the interior angle of a single vertex for a regular polygon with the number of segments
    pub fn regular_interior_angle(p: u8) -> Deg<f64> {
        if p < 2 {
            return Deg(0.0); // This shouldn't happen, but I don't want this code to panic.
        }

        let interior_angle_sum = Deg(180.0) * (p - 2) as f64;
        interior_angle_sum / p as f64
    }

    /// Returns an approximate centroid for a set of segments.
    fn approximate_centroid(segments: &[Segment]) -> Point2<f64> {
        // NOTE: This is not fully accurate for arcs (using midpoint instead of
        //       true centroid). This was done on purpose in MagicTile v1, to
        //       help avoid drawing overlaps. (it biases the calculated centroid
        //       towards large arcs.)
        // - Roice Nelson

        let total_length = segments.iter().map(|seg| seg.length()).sum();

        let weighted_midpoints = segments
            .iter()
            .map(|seg| seg.midpoint().to_vec() * seg.length());
        let sum_of_weighted_midpoints = weighted_midpoints.sum::<Vector2<f64>>();

        Point2::from_vec(sum_of_weighted_midpoints / total_length)
    }
}

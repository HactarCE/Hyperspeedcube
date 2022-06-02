//! Common utilities for implementing 4D twisty puzzles.

/// 4-dimensional axis.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Axis {
    /// X axis (right).
    X = 0,
    /// Y axis (up).
    Y = 1,
    /// Z axis (towards the camera).
    Z = 2,
    /// W axis (towards the 4D camera).
    W = 3,
}
impl Axis {
    /// Returns the perpendicular axes from this one, in the order used for
    /// calculating sticker indices.
    pub fn sticker_order_perpendiculars(self) -> [Axis; 3] {
        use Axis::*;
        // This ordering is necessary in order to maintain compatibility with
        // MC4D sticker indices.
        match self {
            X => [Y, Z, W],
            Y => [X, Z, W],
            Z => [X, Y, W],
            W => [X, Y, Z],
        }
    }
    /// Returns the axes of the oriented plane perpendicular to two other axes.
    pub fn perpendicular_plane(axis1: Axis, axis2: Axis) -> (Axis, Axis) {
        let [t, u, v] = axis1.sticker_order_perpendiculars();
        if axis2 == t {
            (u, v)
        } else if axis2 == u {
            (v, t)
        } else if axis2 == v {
            (t, u)
        } else {
            panic!("no perpendicular plane")
        }
    }
    /// Returns the axis perpendicular to three other axes.
    pub fn perpendicular_axis(axes: [Axis; 3]) -> Axis {
        Axis::iter().find(|ax| !axes.contains(ax)).unwrap()
    }

    /// Returns an iterator over all axes.
    pub fn iter() -> impl Iterator<Item = Axis> {
        [Axis::X, Axis::Y, Axis::Z, Axis::W].into_iter()
    }
}

use super::*;
use crate::math::Rotor;

/// Puzzle twist set metadata.
#[derive(Debug)]
pub struct PuzzleTwists {
    /// Twist set name.
    pub name: String,

    /// Twist axes, in order.
    pub axes: Vec<TwistAxisInfo>,
    /// Twist directions, in order.
    pub directions: Vec<TwistDirectionInfo>,

    /// Puzzle orientations, in no particular order.
    pub orientations: Vec<Rotor>,
}
impl_puzzle_info_trait!(for PuzzleTwists { fn info(TwistAxis) -> &TwistAxisInfo { .axes } });
impl_puzzle_info_trait!(for PuzzleTwists { fn info(TwistDirection) -> &TwistDirectionInfo { .directions } });
impl PuzzleTwists {
    pub fn axis_from_symbol(&self, symbol: &str) -> Option<TwistAxis> {
        (0..self.axes.len() as u8)
            .map(TwistAxis)
            .find(|&twist_axis| self.info(twist_axis).symbol == symbol)
    }
    pub fn direction_from_name(&self, name: &str) -> Option<TwistDirection> {
        (0..self.directions.len() as u8)
            .map(TwistDirection)
            .find(|&twist_direction| self.info(twist_direction).name == name)
    }

    pub fn nearest_orientation(&self, rot: &Rotor) -> Rotor {
        let inv_rot = rot.reverse();

        let mut nearest = Rotor::identity();
        // The scalar part of a rotor is the cosine of half the angle of
        // rotation. So we can use the absolute value of that quantity to
        // compare whether one rotor is a larger rotation than another.
        let mut score_of_nearest = -1.0;
        for candidate in &self.orientations {
            let s = (&inv_rot * candidate).s().abs();

            if s > score_of_nearest {
                nearest = candidate.clone();
                score_of_nearest = s;
            }
        }
        nearest
    }
}

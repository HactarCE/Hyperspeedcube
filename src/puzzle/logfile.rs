use cgmath::Matrix4;
use std::fmt;

pub struct LogFile {
    pub view_matrix: Matrix4,
    pub scramble_moves: Vec<Move>,
    pub solve_moves: Vec<Move>,
}
impl fmt::Display for LogFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MC4D {}")
    }
}

pub struct Move {
    pub sticker: u8,
    pub direction: i8,
    pub layers: u8,
}
impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{},{},{}", self.sticker, self.direction, self.layers)
    }
}

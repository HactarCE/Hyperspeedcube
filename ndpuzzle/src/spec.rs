use crate::math::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BasicPuzzleSpec {
    pub name: String,
    pub ndim: u8,
    pub shape: Vec<ShapeSpec>,
    pub twists: Vec<TwistsSpec>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ShapeSpec {
    pub symmetries: Vec<SymmetriesSpec>,
    pub face_generators: Vec<Vector<f32>>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TwistsSpec {
    pub symmetries: Vec<SymmetriesSpec>,
    pub axes: Vec<AxisSpec>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum SymmetriesSpec {
    Schlafli(String),
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AxisSpec {
    pub normal: Vector<f32>,
    pub cuts: Vec<f32>,
    pub twist_generators: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_3x3x3_spec_deserialize() {
        let s = include_str!("../../puzzles/3x3x3.yaml");
        let _spec: BasicPuzzleSpec = serde_yaml::from_str(s).unwrap();
    }
}

use crate::math::*;
use itertools::Itertools;
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
    pub face_generators: Vec<Vector>,
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
    pub normal: Vector,
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

const AXIS_NAMES: &str = "XYZWUVRS";

pub fn parse_transform(string: &str) -> Option<Matrix> {
    string
        .split("->")
        .map(|v| parse_vector(v)?.normalize())
        .tuple_windows()
        .map(|(v1, v2)| Some(Matrix::from_vec_to_vec(v1.as_ref()?, v2.as_ref()?)))
        .try_fold(Matrix::EMPTY_IDENT, |m1, m2| Some(&m1 * &m2?))
}

pub fn parse_vector(string: &str) -> Option<Vector> {
    if string.contains(',') {
        Some(Vector(
            string
                .split(',')
                .map(|x| x.trim().parse::<f32>())
                .try_collect()
                .ok()?,
        ))
    } else if AXIS_NAMES.contains(string.trim().trim_start_matches('-')) {
        if let Some(s) = string.trim().strip_prefix('-') {
            Some(-Vector::unit(AXIS_NAMES.find(s)? as u8))
        } else {
            Some(Vector::unit(AXIS_NAMES.find(string.trim())? as u8))
        }
    } else {
        None
    }
}

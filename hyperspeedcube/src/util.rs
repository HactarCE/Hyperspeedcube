use ndpuzzle::{math::Vector, vector};
use std::ops::{Add, Mul};

pub const INVALID_STR: &str = "<invalid>";

pub fn wrap_words<S: AsRef<str>>(words: impl Iterator<Item = S>) -> String {
    const WORD_WRAP_WIDTH: usize = 70;
    let mut ret = String::new();
    let mut column = 0;
    for word in words {
        let word = word.as_ref();
        if column == 0 {
            column += word.len();
            ret += word;
        } else {
            column += word.len() + 1;
            if column <= WORD_WRAP_WIDTH {
                ret += " ";
            } else {
                column = word.len();
                ret += "\n";
            }
            ret += word;
        }
    }
    ret
}

pub fn mix<T>(a: T, b: T, t: f32) -> <T::Output as Add>::Output
where
    T: Mul<f32>,
    T::Output: Add,
{
    a * (1.0 - t) + b * t
}

pub(crate) fn from_pt3(p: cgmath::Point3<f32>) -> Vector {
    vector![p.x, p.y, p.z]
}
pub(crate) fn from_vec3(v: cgmath::Vector3<f32>) -> Vector {
    vector![v.x, v.y, v.z]
}
pub(crate) fn from_vec4(v: cgmath::Vector4<f32>) -> Vector {
    vector![v.x, v.y, v.z, v.w]
}

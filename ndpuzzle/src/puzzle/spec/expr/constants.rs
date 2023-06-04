use ahash::AHashMap;

use super::Value;
use crate::math::*;

pub(super) fn builtin_constants() -> AHashMap<&'static str, Value> {
    [
        // Circle constants
        ("π", Value::Number(std::Float::consts::PI)),
        ("pi", Value::Number(std::Float::consts::PI)),
        ("τ", Value::Number(std::Float::consts::TAU)),
        ("tau", Value::Number(std::Float::consts::TAU)),
        // Golden ratio
        ("φ", Value::Number((1.0 + (5.0 as Float).sqrt()) / 2.0)),
        ("phi", Value::Number((1.0 + (5.0 as Float).sqrt()) / 2.0)),
        // Basis vectors
        ("X", Value::Vector(Vector::unit(0))),
        ("Y", Value::Vector(Vector::unit(1))),
        ("Z", Value::Vector(Vector::unit(2))),
        ("W", Value::Vector(Vector::unit(3))),
        ("U", Value::Vector(Vector::unit(4))),
        ("V", Value::Vector(Vector::unit(5))),
    ]
    .into_iter()
    .collect()
}

use super::Value;
use crate::math::*;

lazy_static! {
    pub static ref BUILTIN_CONSTANTS: Vec<(&'static str, Value)> = builtin_constants();
}

fn builtin_constants() -> Vec<(&'static str, Value)> {
    vec![
        // Circle constants
        ("π", Value::Number(std::f32::consts::PI)),
        ("pi", Value::Number(std::f32::consts::PI)),
        ("τ", Value::Number(std::f32::consts::TAU)),
        ("tau", Value::Number(std::f32::consts::TAU)),
        // Golden ratio
        ("φ", Value::Number((1.0 + 5.0_f32.sqrt()) / 2.0)),
        ("phi", Value::Number((1.0 + 5.0_f32.sqrt()) / 2.0)),
        // Basis vectors
        ("X", Value::Vector(Vector::unit(0))),
        ("Y", Value::Vector(Vector::unit(1))),
        ("Z", Value::Vector(Vector::unit(2))),
        ("W", Value::Vector(Vector::unit(3))),
        ("U", Value::Vector(Vector::unit(4))),
        ("V", Value::Vector(Vector::unit(5))),
    ]
}

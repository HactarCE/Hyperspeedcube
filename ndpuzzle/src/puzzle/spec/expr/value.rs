use anyhow::{anyhow, Result};
use std::ops::{Add, Div, Mul, Sub};

use crate::math::*;

#[derive(Debug, Clone)]
pub enum Value {
    Number(f32),
    Vector(Vector),
    Transform(Rotoreflector),
}
impl Value {
    fn type_str(&self) -> &str {
        match self {
            Value::Number(_) => "number",
            Value::Vector(_) => "vector",
            Value::Transform(_) => "transformation",
        }
    }
}

pub struct SpannedValue<'a> {
    span: &'a str,
    value: Value,
}
impl<'a> Add for SpannedValue<'a> {
    type Output = Result<Value>;

    fn add(self, rhs: Self) -> Self::Output {
        match (self.value, rhs.value) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a + b)),
            (Value::Vector(a), Value::Vector(b)) => Ok(Value::Vector(a + b)),
            (self_value, rhs_value) => Err(anyhow!(
                "cannot add {} {:?} by {} {:?}",
                self_value.type_str(),
                self.span,
                rhs_value.type_str(),
                rhs.span,
            )),
        }
    }
}
impl<'a> Sub for SpannedValue<'a> {
    type Output = Result<Value>;

    fn sub(self, rhs: Self) -> Self::Output {
        match (self.value, rhs.value) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a - b)),
            (Value::Vector(a), Value::Vector(b)) => Ok(Value::Vector(a - b)),
            (self_value, rhs_value) => Err(anyhow!(
                "cannot subtract {} {:?} by {} {:?}",
                self_value.type_str(),
                self.span,
                rhs_value.type_str(),
                rhs.span,
            )),
        }
    }
}
impl<'a> Mul for SpannedValue<'a> {
    type Output = Result<Value>;

    fn mul(self, rhs: Self) -> Self::Output {
        match (self.value, rhs.value) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a * b)),
            (Value::Number(n), Value::Vector(v)) => Ok(Value::Vector(v * n)),
            (Value::Vector(v), Value::Number(n)) => Ok(Value::Vector(v * n)),
            (Value::Transform(t), Value::Vector(v)) => Ok(Value::Vector(t * v)),
            (Value::Transform(a), Value::Transform(b)) => Ok(Value::Transform(a * b)),
            (self_value, rhs_value) => Err(anyhow!(
                "cannot multiply {} {:?} by {} {:?}",
                self_value.type_str(),
                self.span,
                rhs_value.type_str(),
                rhs.span,
            )),
        }
    }
}
impl<'a> Div for SpannedValue<'a> {
    type Output = Result<Value>;

    fn div(self, rhs: Self) -> Self::Output {
        match (self.value, rhs.value) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a / b)),
            (Value::Vector(v), Value::Number(n)) => Ok(Value::Vector(v / n)),
            (self_value, rhs_value) => Err(anyhow!(
                "cannot divide {} {:?} by {} {:?}",
                self_value.type_str(),
                self.span,
                rhs_value.type_str(),
                rhs.span,
            )),
        }
    }
}

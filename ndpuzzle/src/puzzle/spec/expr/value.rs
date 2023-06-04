use anyhow::{anyhow, bail, ensure, Result};
use smallvec::{smallvec, SmallVec};
use std::ops::{Add, Div, Mul, Sub};

use crate::math::*;

#[derive(Debug, Clone)]
pub(super) enum Value {
    Number(Float),
    Vector(Vector),
    Transform(Rotoreflector),
}
impl Value {
    pub fn type_str(&self) -> &str {
        match self {
            Value::Number(_) => "number",
            Value::Vector(_) => "vector",
            Value::Transform(_) => "transformation",
        }
    }

    pub fn ensure_finite(&self, span: &str) -> Result<()> {
        let Float_slice = match self {
            Value::Number(n) => std::slice::from_ref(n),
            Value::Vector(v) => &v.0,
            Value::Transform(t) => t.matrix().as_slice(),
        };
        if let Some(bad) = Float_slice.iter().find(|n| !n.is_finite()) {
            bail!("encountered {bad}: {span:?}");
        }
        Ok(())
    }
}

pub(super) struct SpannedValue<'a> {
    pub span: &'a str,
    pub value: Value,
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

impl SpannedValue<'_> {
    pub fn into_vector_elems(self) -> Result<SmallVec<[Float; 4]>> {
        self.ensure_finite()?;
        match self.value {
            Value::Number(n) => Ok(smallvec![n]),
            Value::Vector(v) => Ok(v.0),
            v => Err(anyhow!(
                "cannot construct vector from {}: {:?}",
                v.type_str(),
                self.span,
            )),
        }
    }

    pub fn into_u8(self) -> Result<u8> {
        let span = self.span;

        let n = self.into_number()?;
        let rounded = n.round();
        ensure!(
            approx_eq(&n, &rounded),
            "expected integer; got {n}: {:?}",
            span,
        );
        ensure!(
            (0.0..=256.0 as Float).contains(&rounded),
            "expected positive integer below 256; got {n}: {:?}",
            span,
        );
        Ok(rounded as u8)
    }

    pub fn into_number(self) -> Result<Float> {
        self.ensure_finite()?;
        match self.value {
            Value::Number(n) => Ok(n),
            v => Err(anyhow!(
                "expected number; got {}: {:?}",
                v.type_str(),
                self.span,
            )),
        }
    }

    pub fn into_list_elems(self) -> Result<Vec<Float>> {
        self.ensure_finite()?;
        match self.value {
            Value::Number(x) => Ok(vec![x]),
            v => Err(anyhow!(
                "expected number or list of numbers; got {}: {:?}",
                v.type_str(),
                self.span,
            )),
        }
    }

    pub fn ensure_finite(&self) -> Result<()> {
        self.value.ensure_finite(self.span)
    }

    pub fn unary_plus(self) -> Result<Value> {
        match self.value {
            Value::Number(_) | Value::Vector(_) | Value::Transform(_) => Ok(self.value),
        }
    }
    pub fn unary_minus(self) -> Result<Value> {
        match self.value {
            Value::Number(n) => Ok(Value::Number(-n)),
            Value::Vector(v) => Ok(Value::Vector(-v)),
            Value::Transform(t) => Ok(Value::Transform(t.reverse())),
        }
    }
}

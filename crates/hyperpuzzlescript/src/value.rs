use std::collections::HashMap;

use hypermath::Vector;

use crate::Type;

/// Value in the language.
#[derive(Debug, Default)]
pub enum Value {
    #[default]
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Map(HashMap<String, Value>),
    List(Vec<Value>),
    Fn(TODO),
    Type(Type),

    Vector(Vector),

    EuclidPoint(hypermath::Point),
    EuclidTransform(hypermath::pga::Motor),
    EuclidPlane(hypermath::Hyperplane),
    EuclidRegion(TODO),
}
impl Value {
    pub fn ty(&self) -> Type {
        match self {
            Value::Null => todo!(),
            Value::Bool(_) => todo!(),
            Value::Number(_) => todo!(),
            Value::String(_) => todo!(),
            Value::Map(hash_map) => todo!(),
            Value::List(values) => todo!(),
            Value::Fn(todo) => todo!(),
            Value::Type(_) => todo!(),
            Value::Vector(vector) => todo!(),
            Value::EuclidPoint(point) => todo!(),
            Value::EuclidTransform(motor) => todo!(),
            Value::EuclidPlane(hyperplane) => todo!(),
            Value::EuclidRegion(todo) => todo!(),
        }
    }

    pub fn unwrap_num(&self) -> f64 {
        match self {
            Value::Number(n) => *n,
            _ => panic!("expected number"),
        }
    }
}

// TODO: delete this
#[derive(Debug)]
struct TODO;

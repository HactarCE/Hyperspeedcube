use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::fmt;

pub mod ast;
mod constants;
mod ctx;
mod functions;
mod parser;
mod value;

use constants::BUILTIN_CONSTANTS;
use functions::{Function, BUILTIN_FUNCTIONS};
pub use parser::parse_expression;
pub use value::*;

// TODO: consolidate constants with this name
const AXIS_NAMES: &str = "XYZWUVRS";

#[derive(Serialize, Debug, Default, Clone, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct MathExpr(String);
impl fmt::Display for MathExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl<'de> serde::Deserialize<'de> for MathExpr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(SerdeMathExpr::deserialize(deserializer)?.into())
    }
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum SerdeMathExpr {
    String(String),
    List(Vec<SerdeMathExpr>),
}
impl From<SerdeMathExpr> for MathExpr {
    fn from(value: SerdeMathExpr) -> Self {
        MathExpr(match value {
            SerdeMathExpr::String(string) => string,
            SerdeMathExpr::List(list) => {
                let mut exprs = list.into_iter().map(MathExpr::from);
                format!("[{}]", exprs.join(", "))
            }
        })
    }
}

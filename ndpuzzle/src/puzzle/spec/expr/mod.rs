use anyhow::{anyhow, bail, ensure, Context, Result};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::fmt;

mod ast;
mod constants;
mod env;
mod functions;
mod parser;
mod value;

use env::Env;
use functions::Function;
pub use value::*;

/// Math expression.
#[derive(Serialize, Debug, Default, Clone, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct MathExpr(pub String);
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
impl MathExpr {
    pub fn compile<'a>(&'a self) -> Result<CompiledMathExpr<'a>> {
        parser::parse_expression(&self.0).map(CompiledMathExpr)
    }
}

#[derive(Debug, Clone)]
pub struct CompiledMathExpr<'a>(ast::ExprAst<'a>);
impl<'a> CompiledMathExpr<'a> {
    fn eval_assign(&self, env: &mut Env<'a>, name: &'a str) -> Result<()> {
        if env.constants.contains_key(name) {
            bail!("cannot redefine constant {name:?}");
        }
        let value = self.eval(env)?.value;
        env.constants.insert(name, value);
        Ok(())
    }

    fn eval<'b>(&'b self, env: &Env<'b>) -> Result<SpannedValue<'b>> {
        self.0.eval(env)
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

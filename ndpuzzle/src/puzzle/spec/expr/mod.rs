//! Mathematical expression parsing and evaluation.

use anyhow::{bail, Result};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::fmt;

mod ast;
mod constants;
mod env;
mod functions;
mod parser;
mod value;

pub use env::Env;
use functions::Function;
use value::{SpannedValue, Value};

/// User-written string representing a mathematical expression.
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
    /// Compiles an expression, checking for syntax errors but not semantic
    /// errors.
    pub fn compile<'a>(&'a self) -> Result<CompiledMathExpr<'a>> {
        parser::parse_expression(&self.0).map(CompiledMathExpr)
    }
}

/// Compiled form of a mathematical expression, which can be directly evaluated.
#[derive(Debug, Clone)]
pub struct CompiledMathExpr<'a>(ast::ExprAst<'a>);
impl<'a> CompiledMathExpr<'a> {
    /// Registers a function `func_name` in `env`.
    pub fn register_as_function(
        self,
        env: &mut Env<'a>,
        func_name: &'a str,
        arg_names: Vec<String>,
    ) {
        env.functions.insert(
            func_name,
            Function::Custom {
                arg_names,
                body: self.0,
            },
        );
    }

    /// Evaluates an expression and assigns it to `name` in `env`.
    pub fn eval_assign(&self, env: &mut Env<'a>, name: &'a str) -> Result<()> {
        if env.constants.contains_key(name) {
            bail!("cannot redefine constant {name:?}");
        }
        let value = self.eval(env)?.value;
        env.constants.insert(name, value);
        Ok(())
    }

    /// Evaluates an expression and returns the resulting value.
    fn eval<'b>(&'b self, env: &Env<'b>) -> Result<SpannedValue<'b>> {
        self.0.eval(env)
    }

    /// Evaluates an expression and coerces it to a number.
    pub fn eval_number(&self, env: &Env<'a>) -> Result<Float> {
        self.eval(env)?.into_number()
    }
    /// Evaluates an expression as a number or list of numbers.
    pub fn eval_list(&self, env: &Env<'a>) -> Result<Vec<Float>> {
        self.0.eval_list(env)
    }
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum SerdeMathExpr {
    String(String),
    Number(Float),
    List(Vec<SerdeMathExpr>),
}
impl From<SerdeMathExpr> for MathExpr {
    fn from(value: SerdeMathExpr) -> Self {
        MathExpr(match value {
            SerdeMathExpr::String(string) => string,
            SerdeMathExpr::Number(n) => n.to_string(),
            SerdeMathExpr::List(list) => {
                let mut exprs = list.into_iter().map(MathExpr::from);
                format!("[{}]", exprs.join(", "))
            }
        })
    }
}

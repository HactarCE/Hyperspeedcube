use anyhow::{anyhow, bail, Result};
use itertools::Itertools;

use super::{Env, SpannedValue, Value};

#[derive(Debug, Clone)]
pub(super) struct ExprAst<'a> {
    pub span: &'a str,
    pub node: ExprAstNode<'a>,
}
impl<'a> ExprAst<'a> {
    pub fn eval(&self, env: &Env<'a>) -> Result<SpannedValue> {
        let value = match &self.node {
            ExprAstNode::Number(n) => Value::Number(*n),
            ExprAstNode::Identifier(name) => env
                .constants
                .get(name)
                .ok_or_else(|| anyhow!("undefined constant: {name:?}"))?
                .clone(),
            ExprAstNode::FuncCall(func_name, args) => {
                let function = env
                    .functions
                    .get(func_name)
                    .ok_or_else(|| anyhow!("undefined function: {func_name:?}"))?;
                let arg_values = args.iter().map(|arg| arg.eval(env)).try_collect()?;

                function.call(env, self.span, arg_values)?
            }
            ExprAstNode::Paren(contents) => contents.eval(env)?.value,
            ExprAstNode::Vector(elements) => {
                let element_values = elements
                    .iter()
                    .map(|arg| arg.eval(env)?.into_vector_elems())
                    .flatten_ok()
                    .try_collect()?;

                Value::Vector(element_values)
            }
            ExprAstNode::BinaryOp { lhs, op, rhs } => {
                let lhs = lhs.eval(env)?;
                let rhs = rhs.eval(env)?;
                match op {
                    BinaryOp::Add => (lhs + rhs)?,
                    BinaryOp::Sub => (lhs - rhs)?,
                    BinaryOp::Mul => (lhs * rhs)?,
                    BinaryOp::Div => (lhs / rhs)?,
                    BinaryOp::Pow => todo!(),
                    BinaryOp::Accessor => todo!(),
                    BinaryOp::Conj => todo!(),
                    BinaryOp::Rotate => todo!(),
                    BinaryOp::Reflect => todo!(),
                    BinaryOp::ByAngle => todo!(),
                }
            }
            ExprAstNode::UnaryOp { op, arg } => {
                let arg = arg.eval(env)?;
                match op {
                    UnaryOp::Pos => arg.unary_plus()?,
                    UnaryOp::Neg => arg.unary_minus()?,
                }
            }
            ExprAstNode::Range { .. } => {
                bail!("range construct is not allowed here: {:?}", self.span)
            }
        };

        Ok(SpannedValue {
            span: self.span,
            value,
        })
    }

    pub fn eval_list(&self, env: &Env<'a>) -> Result<Vec<Float>> {
        match &self.node {
            ExprAstNode::Vector(exprs) => exprs
                .iter()
                .map(|expr| expr.eval_list_elems(env))
                .flatten_ok()
                .collect(),

            _ => self.eval_list_elems(env),
        }
    }

    fn eval_list_elems(&self, env: &Env<'a>) -> Result<Vec<Float>> {
        match &self.node {
            ExprAstNode::Range { count, from, to } => {
                let count = count.eval(env)?.into_u8()?;
                let from = from.eval(env)?.into_number()?;
                let to = to.eval(env)?.into_number()?;

                Ok((0..count)
                    .map(|i| (i + 1) as Float / (count + 1) as Float)
                    .map(|t| crate::math::util::mix(from, to, t))
                    .collect())
            }

            _ => self.eval(env)?.into_list_elems(),
        }
    }
}

#[derive(Debug, Clone)]
pub(super) enum ExprAstNode<'a> {
    Number(Float),
    Identifier(&'a str),
    FuncCall(&'a str, Vec<ExprAst<'a>>),
    Paren(Box<ExprAst<'a>>),
    Vector(Vec<ExprAst<'a>>),
    BinaryOp {
        lhs: Box<ExprAst<'a>>,
        op: BinaryOp,
        rhs: Box<ExprAst<'a>>,
    },
    UnaryOp {
        op: UnaryOp,
        arg: Box<ExprAst<'a>>,
    },
    Range {
        count: Box<ExprAst<'a>>,
        from: Box<ExprAst<'a>>,
        to: Box<ExprAst<'a>>,
    },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,

    /// Property accessor.
    Accessor,

    /// Conjunction of transform conditions.
    Conj,

    /// Rotation operator.
    Rotate,
    /// Reflection operator.
    Reflect,
    /// Rotation angle adjustment operator.
    ByAngle,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    Pos,
    Neg,
}

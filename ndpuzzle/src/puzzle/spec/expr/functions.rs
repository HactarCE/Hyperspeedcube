use ahash::AHashMap;
use anyhow::{ensure, Result};
use std::fmt;
use std::ops::RangeInclusive;
use std::sync::Arc;

use super::{ast::ExprAst, Env, SpannedValue, Value};

#[derive(Clone)]
pub enum Function<'a> {
    Builtin {
        arg_count_range: RangeInclusive<usize>,
        func: Arc<dyn for<'b> Fn(&Env<'b>, &'b str, Vec<SpannedValue<'b>>) -> Result<Value>>,
    },
    Custom {
        arg_names: Vec<String>,
        body: ExprAst<'a>,
    },
}
impl fmt::Debug for Function<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Builtin {
                arg_count_range,
                func,
            } => f
                .debug_struct("Builtin")
                .field("arg_count_range", arg_count_range)
                .finish(),
            Self::Custom { arg_names, body } => f
                .debug_struct("Custom")
                .field("arg_names", arg_names)
                .field("body", body)
                .finish(),
        }
    }
}

impl<'a> Function<'a> {
    fn expected_arg_count(&self) -> RangeInclusive<usize> {
        match self {
            Function::Builtin {
                arg_count_range, ..
            } => arg_count_range.clone(),
            Function::Custom { arg_names, .. } => arg_names.len()..=arg_names.len(),
        }
    }

    pub fn call(&self, env: &Env<'a>, span: &'a str, args: Vec<SpannedValue<'a>>) -> Result<Value> {
        let expected_arg_count = self.expected_arg_count();
        ensure!(
            expected_arg_count.contains(&args.len()),
            "invalid arg count; expected {:?} but got {}: {:?}",
            expected_arg_count,
            args.len(),
            span,
        );

        match self {
            Function::Builtin {
                arg_count_range,
                func,
            } => func(env, span, args),
            Function::Custom { arg_names, body } => {
                let arg_table = arg_names
                    .iter()
                    .zip(args)
                    .map(|(name, value)| (name.as_str(), value.value))
                    .collect();

                // Create a new environment with the function parameters.
                let child_env = Env {
                    ndim: env.ndim,
                    functions: AHashMap::new(),
                    constants: arg_table,
                    parent: Some(env.base()),
                };
                // Call function.
                Ok(body.eval(&child_env)?.value)
            }
        }
    }

    fn unary_numeric(f: fn(f32) -> f32) -> Self {
        Self::Builtin {
            arg_count_range: 1..=1,
            func: Arc::new(move |_env, span, mut args| {
                let arg = args.remove(0).into_number()?;
                let ret = Value::Number(f(arg));
                ret.ensure_finite(span)?;
                Ok(ret)
            }),
        }
    }

    pub fn builtins() -> AHashMap<&'a str, Self> {
        [
            // Roots
            ("sqrt", Self::unary_numeric(|n| n.sqrt())),
            ("cbrt", Self::unary_numeric(|n| n.cbrt())),
            // Trigonometry
            ("sin", Self::unary_numeric(|n| n.sin())),
            ("cos", Self::unary_numeric(|n| n.cos())),
            ("tan", Self::unary_numeric(|n| n.tan())),
            ("asin", Self::unary_numeric(|n| n.asin())),
            ("acos", Self::unary_numeric(|n| n.acos())),
            ("atan", Self::unary_numeric(|n| n.atan())),
            ("sec", Self::unary_numeric(|n| n.sin().recip())),
            ("csc", Self::unary_numeric(|n| n.cos().recip())),
            ("cot", Self::unary_numeric(|n| n.tan().recip())),
            ("asec", Self::unary_numeric(|n| n.recip().asin())),
            ("acsc", Self::unary_numeric(|n| n.recip().acos())),
            ("acot", Self::unary_numeric(|n| n.recip().atan())),
            // Hyperbolic trigonometry
            ("sinh", Self::unary_numeric(|n| n.sinh())),
            ("cosh", Self::unary_numeric(|n| n.cosh())),
            ("tanh", Self::unary_numeric(|n| n.tanh())),
            ("asinh", Self::unary_numeric(|n| n.asinh())),
            ("acosh", Self::unary_numeric(|n| n.acosh())),
            ("atanh", Self::unary_numeric(|n| n.atanh())),
            ("sech", Self::unary_numeric(|n| n.sinh().recip())),
            ("csch", Self::unary_numeric(|n| n.cosh().recip())),
            ("coth", Self::unary_numeric(|n| n.tanh().recip())),
            ("asech", Self::unary_numeric(|n| n.recip().asinh())),
            ("acsch", Self::unary_numeric(|n| n.recip().acosh())),
            ("acoth", Self::unary_numeric(|n| n.recip().atanh())),
            // // Transformations
            // ("reflect", Arc::new(Reflect)),
            // //
            // ("plane", Arc::new(Plane)),
            // ("plane_from_points", Arc::new(PlaneFromPoints)),
            // //
            // ("sphere", Arc::new(Sphere)),
            // ("sphere_from_points", Arc::new(SphereFromPoints)),
            // //
            // ("line", Arc::new(Line)),
            // ("line_from_points", Arc::new(LineFromPoints)),
            // //
            // ("intersect", Arc::new(Intersect)),
        ]
        .into_iter()
        .collect()
    }
}

// struct Plane;
// impl Function<'static> for Plane {
//     fn arg_count_range<'a>(&self, env: &Env<'a>) -> RangeInclusive<usize> {
//         1..=1
//     }

//     fn call<'a>(
//         &self,
//         env: &Env<'a>,
//         span: &'a str,
//         args: &[SpannedValue<'a>],
//     ) -> Result<Value> {
//         todo!()
//     }
// }

// struct PlaneFromPoints;
// impl Function<'static> for PlaneFromPoints {
//     fn arg_count_range<'a>(&self, env: &Env<'a>) -> RangeInclusive<usize> {
//         1..=env.ndim() as usize
//     }

//     fn call<'a>(
//         &self,
//         env: &Env<'a>,
//         span: &'a str,
//         args: &[SpannedValue<'a>],
//     ) -> Result<Value> {
//         todo!()
//     }
// }

// struct Sphere;
// impl Function<'static> for Sphere {
//     fn arg_count_range<'a>(&self, env: &Env<'a>) -> RangeInclusive<usize> {
//         1..=1
//     }

//     fn call<'a>(
//         &self,
//         env: &Env<'a>,
//         span: &'a str,
//         args: &[SpannedValue<'a>],
//     ) -> Result<Value> {
//         todo!()
//     }
// }

// struct SphereFromPoints;
// impl Function<'static> for SphereFromPoints {
//     fn arg_count_range<'a>(&self, env: &Env<'a>) -> RangeInclusive<usize> {
//         1..=env.ndim() as usize
//     }

//     fn call<'a>(
//         &self,
//         env: &Env<'a>,
//         span: &'a str,
//         args: &[SpannedValue<'a>],
//     ) -> Result<Value> {
//         todo!()
//     }
// }

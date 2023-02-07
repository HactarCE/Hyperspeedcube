use anyhow::Result;
use std::fmt;
use std::ops::RangeInclusive;
use std::sync::Arc;

use super::{ctx::Ctx, SpannedValue};
use crate::math::*;

pub trait Function: fmt::Debug + Send + Sync {
    fn arg_count_range<'a>(&self, ctx: &Ctx<'a>) -> RangeInclusive<usize>;

    fn call<'a>(
        &self,
        ctx: &Ctx<'a>,
        span: &'a str,
        args: &[SpannedValue<'a>],
    ) -> Result<SpannedValue<'a>>;
}

lazy_static! {
    pub static ref BUILTIN_FUNCTIONS: Vec<(&'static str, Arc<dyn Function>)> = builtin_functions();
}

fn builtin_functions() -> Vec<(&'static str, Arc<dyn Function>)> {
    vec![
        // Roots
        ("sqrt", Arc::new(UnaryNumeric(|n| n.sqrt()))),
        ("cbrt", Arc::new(UnaryNumeric(|n| n.cbrt()))),
        // Trigonometry
        ("sin", Arc::new(UnaryNumeric(|n| n.sin()))),
        ("cos", Arc::new(UnaryNumeric(|n| n.cos()))),
        ("tan", Arc::new(UnaryNumeric(|n| n.tan()))),
        ("asin", Arc::new(UnaryNumeric(|n| n.asin()))),
        ("acos", Arc::new(UnaryNumeric(|n| n.acos()))),
        ("atan", Arc::new(UnaryNumeric(|n| n.atan()))),
        ("sec", Arc::new(UnaryNumeric(|n| n.sin().recip()))),
        ("csc", Arc::new(UnaryNumeric(|n| n.cos().recip()))),
        ("cot", Arc::new(UnaryNumeric(|n| n.tan().recip()))),
        ("asec", Arc::new(UnaryNumeric(|n| n.recip().asin()))),
        ("acsc", Arc::new(UnaryNumeric(|n| n.recip().acos()))),
        ("acot", Arc::new(UnaryNumeric(|n| n.recip().atan()))),
        // Hyperbolic trigonometry
        ("sinh", Arc::new(UnaryNumeric(|n| n.sinh()))),
        ("cosh", Arc::new(UnaryNumeric(|n| n.cosh()))),
        ("tanh", Arc::new(UnaryNumeric(|n| n.tanh()))),
        ("asinh", Arc::new(UnaryNumeric(|n| n.asinh()))),
        ("acosh", Arc::new(UnaryNumeric(|n| n.acosh()))),
        ("atanh", Arc::new(UnaryNumeric(|n| n.atanh()))),
        ("sech", Arc::new(UnaryNumeric(|n| n.sinh().recip()))),
        ("csch", Arc::new(UnaryNumeric(|n| n.cosh().recip()))),
        ("coth", Arc::new(UnaryNumeric(|n| n.tanh().recip()))),
        ("asech", Arc::new(UnaryNumeric(|n| n.recip().asinh()))),
        ("acsch", Arc::new(UnaryNumeric(|n| n.recip().acosh()))),
        ("acoth", Arc::new(UnaryNumeric(|n| n.recip().atanh()))),
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
}

#[derive(Debug, Copy, Clone)]
struct UnaryNumeric(fn(f32) -> f32);
impl Function for UnaryNumeric {
    fn arg_count_range<'a>(&self, ctx: &Ctx<'a>) -> RangeInclusive<usize> {
        1..=1
    }

    fn call<'a>(
        &self,
        ctx: &Ctx<'a>,
        span: &'a str,
        args: &[SpannedValue<'a>],
    ) -> Result<SpannedValue<'a>> {
        // TODO: handle NaN as well
        todo!()
    }
}

// struct Plane;
// impl Function for Plane {
//     fn arg_count_range<'a>(&self, ctx: &Ctx<'a>) -> RangeInclusive<usize> {
//         1..=1
//     }

//     fn call<'a>(
//         &self,
//         ctx: &Ctx<'a>,
//         span: &'a str,
//         args: &[SpannedValue<'a>],
//     ) -> Result<SpannedValue<'a>> {
//         todo!()
//     }
// }

// struct PlaneFromPoints;
// impl Function for PlaneFromPoints {
//     fn arg_count_range<'a>(&self, ctx: &Ctx<'a>) -> RangeInclusive<usize> {
//         1..=ctx.ndim() as usize
//     }

//     fn call<'a>(
//         &self,
//         ctx: &Ctx<'a>,
//         span: &'a str,
//         args: &[SpannedValue<'a>],
//     ) -> Result<SpannedValue<'a>> {
//         todo!()
//     }
// }

// struct Sphere;
// impl Function for Sphere {
//     fn arg_count_range<'a>(&self, ctx: &Ctx<'a>) -> RangeInclusive<usize> {
//         1..=1
//     }

//     fn call<'a>(
//         &self,
//         ctx: &Ctx<'a>,
//         span: &'a str,
//         args: &[SpannedValue<'a>],
//     ) -> Result<SpannedValue<'a>> {
//         todo!()
//     }
// }

// struct SphereFromPoints;
// impl Function for SphereFromPoints {
//     fn arg_count_range<'a>(&self, ctx: &Ctx<'a>) -> RangeInclusive<usize> {
//         1..=ctx.ndim() as usize
//     }

//     fn call<'a>(
//         &self,
//         ctx: &Ctx<'a>,
//         span: &'a str,
//         args: &[SpannedValue<'a>],
//     ) -> Result<SpannedValue<'a>> {
//         todo!()
//     }
// }

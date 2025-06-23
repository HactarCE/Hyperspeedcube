use hypermath::prelude::*;

use crate::{FnType, Map, Num, Result, Scope, Type, builtins::vec::construct_from_args};

pub fn define_in(scope: &Scope) -> Result<()> {
    scope.register_builtin_functions(hps_fns![
        /// `point()` constructs a [point](#points) in Euclidean space and can
        /// be called in any of several ways:
        ///
        /// - **No arguments.** Calling `point()` with no arguments returns the
        ///   origin.
        /// - **Positional arguments.** Calling `point()` with multiple numbers
        ///   constructs a point with those components. For example, `point(10,
        ///   20, 30)` constructs the point at $\langle 10, 20, 30 \rangle$.
        /// - **Named arguments.** Calling `point()` with named arguments
        ///   constructs a point with those components. The names must use the
        ///   same mapping as [vector component
        ///   access](#vector-component-access). For example, `point(x=10,
        ///   z=30)` constructs the point at $\langle 10, 0, 30 \rangle$.
        /// - **Vector.** Calling `point()` with an existing vector will return
        ///   the point that is at that displacement from the origin.
        /// - **Point.** Calling `point()` with an existing point will return
        ///   the point unmodified.
        #[kwargs(kwargs)]
        fn point(ctx: EvalCtx) -> Point {
            Point(construct_from_args(ctx.caller_span, &[], kwargs)?)
        }
        #[fn_type(FnType { params: vec![Type::Num], is_variadic: true, ret: Type::EuclidPoint })]
        fn point(ctx: EvalCtx, args: Args) -> Point {
            Point(construct_from_args(ctx.caller_span, &args, Map::new())?)
        }
        #[fn_type(FnType { params: vec![Type::Vec], is_variadic: false, ret: Type::EuclidPoint })]
        fn point(ctx: EvalCtx, args: Args) -> Point {
            Point(construct_from_args(ctx.caller_span, &args, Map::new())?)
        }
        #[fn_type(FnType { params: vec![Type::EuclidPoint], is_variadic: false, ret: Type::EuclidPoint })]
        fn point(ctx: EvalCtx, args: Args) -> Point {
            Point(construct_from_args(ctx.caller_span, &args, Map::new())?)
        }
    ])?;

    scope.register_builtin_functions(hps_fns![
        // Operators
        ("+", |_, a: Point, b: Vector| -> Point { a + b }),
        ("+", |_, a: Vector, b: Point| -> Point { b + a }),
        ("-", |_, a: Point, b: Vector| -> Point { a - b }),
        ("-", |_, a: Point, b: Point| -> Vector { a - b }),
        // Interpolation
        ("lerp", |_, a: Point, b: Point, t: Num| -> Point {
            Point(hypermath::util::lerp(a.0, b.0, t.clamp(0.0, 1.0)))
        }),
        ("lerp_unbounded", |_, a: Point, b: Point, t: Num| -> Point {
            Point(hypermath::util::lerp(a.0, b.0, t))
        }),
        // Other functions
        ("distance", |_, a: Point, b: Point| -> Num {
            (a - b).mag2()
        }),
    ])
}

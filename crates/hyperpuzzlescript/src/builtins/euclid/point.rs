use hypermath::prelude::*;

use crate::{Result, Scope};

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
        fn point(ctx: EvalCtx, args: Args) -> EuclidPoint {
            let v = crate::builtins::geometry::construct_vec(ctx.caller_span, &args, kwargs)?;
            Point(v)
        }
    ])?;

    scope.register_builtin_functions([
        // Operators
        hps_fn!("+", |a: EuclidPoint, b: Vec| -> EuclidPoint { a + b }),
        hps_fn!("+", |a: Vec, b: EuclidPoint| -> EuclidPoint { b + a }),
        hps_fn!("-", |a: EuclidPoint, b: Vec| -> EuclidPoint { a - b }),
        hps_fn!("-", |a: EuclidPoint, b: EuclidPoint| -> Vec { a - b }),
        // Interpolation
        hps_fn!("lerp", |a: EuclidPoint,
                         b: EuclidPoint,
                         t: Num|
         -> EuclidPoint {
            Point(hypermath::util::lerp(a.0, b.0, t.clamp(0.0, 1.0)))
        }),
        hps_fn!("lerp_unbounded", |a: EuclidPoint,
                                   b: EuclidPoint,
                                   t: Num|
         -> EuclidPoint {
            Point(hypermath::util::lerp(a.0, b.0, t))
        }),
        // Other functions
        hps_fn!("distance", |a: EuclidPoint, b: EuclidPoint| -> Num {
            (a - b).mag2()
        }),
    ])
}

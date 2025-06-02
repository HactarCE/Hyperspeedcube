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
        fn point(_ctx: EvalCtx, args: Args) -> EuclidPoint {
            crate::builtins::geometry::vec::construct_vec(&args, kwargs).map(Point)?
        }
    ])?;

    scope.register_builtin_functions([
        // Operators
        hps_fn!("+", |a: EPoint, b: Vec| -> EPoint { a + b }),
        hps_fn!("+", |a: Vec, b: EPoint| -> EPoint { b + a }),
        hps_fn!("-", |a: EPoint, b: Vec| -> EPoint { a - b }),
        hps_fn!("-", |a: EPoint, b: EPoint| -> Vec { a - b }),
        // Interpolation
        hps_fn!("lerp", |a: EPoint, b: EPoint, t: Num| -> EPoint {
            Point(hypermath::util::lerp(a.0, b.0, t.clamp(0.0, 1.0)))
        }),
        hps_fn!("lerp_unbounded", |a: EPoint, b: EPoint, t: Num| -> EPoint {
            Point(hypermath::util::lerp(a.0, b.0, t))
        }),
        // Other functions
        hps_fn!("distance", |a: EPoint, b: EPoint| -> Num { (a - b).mag2() }),
    ])
}

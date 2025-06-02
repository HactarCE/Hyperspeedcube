use hypermath::prelude::*;

use crate::{Error, Result, Scope};

pub fn define_in(scope: &Scope) -> Result<()> {
    scope.register_builtin_functions(hps_fns![
        /// `vec()` constructs a [vector](#vectors) and can be called in any of
        /// several ways:
        ///
        /// - **No arguments.** Calling `vec()` with no arguments returns the
        ///   zero vector. For example, `vec()` constructs the vector $\langle
        ///   0, 0, 0 \rangle$.
        /// - **Positional arguments.** Calling `vec()` with multiple numbers
        ///   constructs a vector with those components. For example, `vec(10,
        ///   20, 30)` constructs the vector $\langle 10, 20, 30 \rangle$.
        /// - **Named arguments.** Calling `vec()` with named arguments
        ///   constructs a vector with those components. The names must use the
        ///   same mapping as [vector component
        ///   access](#vector-component-access). For example, `vec(x=10, z=30)`
        ///   constructs the blade $10x+30y$, which represents the vector
        ///   $\langle 10, 0, 30 \rangle$.
        /// - **Vector.** Calling `vec()` with an existing vector will return
        ///   the vector unmodified.
        /// - **Point.** Calling `vec()` with an existing point will return its
        ///   coordinates as a vector.
        #[kwargs(kwargs)]
        fn vec(ctx: EvalCtx, args: Args) -> Vec {
            super::construct_vec(ctx.caller_span, &args, kwargs)?
        }

        /// `dot()` returns the [dot product] between two vectors.
        ///
        /// [dot product]:
        ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Dot_products#Dot_Product
        fn dot(a: Vec, b: Vec) -> Vec {
            a.dot(b)
        }

        /// `cross()` returns the 3D cross product between the two vectors. It
        /// returns an error if either vector has components outside the XYZ
        /// subspace.
        fn cross((a, a_span): Vec, (b, b_span): Vec) -> Vec {
            for (v, v_span) in [(&a, a_span), (&b, b_span)] {
                if v.iter_nonzero().any(|(i, _)| i >= 3) {
                    let msg = "cross product is undefined beyond 3D";
                    return Err(Error::bad_arg(v.clone(), Some(msg)).at(v_span));
                }
            }
            a.cross_product_3d(b)
        }

        /// `lerp()` returns the linear interpolation between two vectors `a`
        /// and `b`, computed as $a (1-t) + b t$ where $t$ is clamped between
        /// `0` and `1`
        fn lerp(a: Vec, b: Vec, t: Num) -> Vec {
            hypermath::util::lerp(a, b, t.clamp(0.0, 1.0))
        }

        /// `lerp_unbounded()` returns the unbounded linear interpolation
        /// between two vectors `a` and `b`, computed as $a (1-t) + b t$.
        fn lerp_unbounded(a: Vec, b: Vec, t: Num) -> Vec {
            hypermath::util::lerp(a, b, t)
        }
    ])?;

    // Operators
    scope.register_builtin_functions(hps_short_fns![
        ("+", |_, v: Vec| -> Vec { v }),
        ("-", |_, v: Vec| -> Vec { -v }),
        ("+", |_, a: Vec, b: Vec| -> Vec { a + b }),
        ("-", |_, a: Vec, b: Vec| -> Vec { a - b }),
        ("*", |_, v: Vec, n: Num| -> Vec { v * n }),
        ("*", |_, n: Num, v: Vec| -> Vec { v * n }),
        ("/", |_, v: Vec, n: Num| -> Vec { v / n }),
    ])
}

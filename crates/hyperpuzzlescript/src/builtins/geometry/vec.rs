use ecow::eco_format;
use hypermath::prelude::*;
use indexmap::IndexMap;

use crate::{Error, Key, Result, Scope, Span, Type, Value, ValueData};

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
            construct_vec(ctx.caller_span, &args, kwargs)?
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

pub(super) fn construct_vec(
    span: Span,
    args: &[Value],
    kwargs: IndexMap<Key, Value>,
) -> Result<Vector> {
    match args {
        [] => {
            unpack_kwargs!(
                kwargs,
                x: Num = 0.0,
                y: Num = 0.0,
                z: Num = 0.0,
                w: Num = 0.0,
                v: Num = 0.0,
                u: Num = 0.0,
                t: Num = 0.0,
            );
            let mut ret = vector![];
            for (i, n) in [x, y, z, w, v, u, t].iter().enumerate() {
                if is_approx_nonzero(&n) {
                    ret.resize_and_set(i as u8, n);
                }
            }
            Ok(ret)
        }

        [arg] => match &arg.data {
            ValueData::Num(n) => Ok(vector![*n]),
            ValueData::Vec(v) => Ok(v.clone()),
            ValueData::EuclidPoint(p) => Ok(p.0.clone()),
            _ => Err(arg.type_error(Type::Union(vec![Type::Num, Type::Vec, Type::EuclidPoint]))),
        },

        _ if args.len() > hypermath::MAX_NDIM as usize => Err(Error::User(eco_format!(
            "vector too long (max is {})",
            hypermath::MAX_NDIM,
        ))
        .at(span)),

        _ => args.iter().map(|arg| arg.as_num()).collect(),
    }
}

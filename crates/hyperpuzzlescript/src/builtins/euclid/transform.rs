use std::borrow::Cow;

use hypermath::pga::*;
use hypermath::prelude::*;

use crate::{Error, ErrorExt, Num, Result, Scope};

pub fn define_in(scope: &Scope) -> Result<()> {
    scope.register_builtin_functions(hps_fns![
        /// `ident()` constructs the identity transformation. It requires
        /// `#ndim` to be defined.
        fn ident(ctx: EvalCtx) -> Motor {
            Motor::ident(ctx.ndim()?)
        }

        /// `refl()` constructs a transform representing a [reflection] or
        /// [point reflection] and can be called in any of several ways:
        ///
        /// - **No arguments.** Calling `refl()` with no arguments constructs a
        ///   point reflection across the origin.
        /// - **Point.** Calling `refl()` with a [point] constructs a point
        ///   reflection across that point. For example, `refl(point(0, -2))`
        ///   constructs a point reflection across the point $\langle 0, -2, 0
        ///   \rangle$.
        /// - **Vector.** Calling `refl()` with a [vector] constructs a
        ///   reflection through that vector. The magnitude of the vector is
        ///   ignored. For example, `refl(point(0, -2))` constructs a reflection
        ///   across the plane $y=0$.
        /// - **Hyperplane.** Calling `refl()` with a [hyperplane] constructs a
        ///   reflection across that hyperplane. The orientation of the plane is
        ///   ignored. For example, `refl(plane('z', 1/2))` constructs a
        ///   reflection across the plane $z = 0.5$.
        fn refl(ctx: EvalCtx) -> Motor {
            Motor::point_reflection(ctx.ndim()?, &Point::ORIGIN)
                .ok_or(Error::User("dimension mismatch".into()).at(ctx.caller_span))?
        }
        fn refl(ctx: EvalCtx, (p, p_span): Point) -> Motor {
            Motor::point_reflection(ctx.ndim()?, &p)
                .ok_or(Error::User("dimension mismatch".into()).at(p_span))?
        }
        fn refl(ctx: EvalCtx, (v, v_span): Vector) -> Motor {
            ctx.ndim()?; // for consistency
            Motor::vector_reflection(v)
                .ok_or(Error::User("dimension mismatch".into()).at(v_span))?
        }
        fn refl(ctx: EvalCtx, (h, h_span): Hyperplane) -> Motor {
            Motor::plane_reflection(ctx.ndim()?, &h)
                .ok_or(Error::User("dimension mismatch".into()).at(h_span))?
        }

        /// `rot()` constructs a transform representing a [rotation] fixing the
        /// origin. It requires `#ndim` to be defined.
        ///
        /// In 2D, `rot()` can be called with a `angle` as a positional
        /// argument, which constructs a counterclockwise rotation by `angle`.
        /// In any dimension, it can be called with the following optional named
        /// arguments:
        ///
        /// - **`from`** is a vector. The magnitude of the vector is ignored.
        /// - **`to`** is the destination vector for `from` once the rotation
        ///   has been applied. The magnitude of the vector is ignored.
        /// - **`fix`** is a [blade] to keep fixed during the rotation.
        /// - **`angle`** is the angle of the rotation in radians.
        ///
        /// Any combination of these may be specified, subject to the following
        /// constraints:
        ///
        /// <div class="annotate" markdown>
        ///
        /// - `from` and `to` are mutually dependent; i.e., one cannot be
        ///   specified without the other.
        /// - If `from` and `to` are not specified, then `fix` and `angle` are
        ///   both required and `fix` must be dual to a 2D plane. (1)
        /// - `from` and `to` must not be opposite each other. (2)
        ///
        /// If `from` and `to` are specified, then the rotation is constructed
        /// using these steps:
        ///
        /// 1. If `fix` is specified, then `from` and `to` are [orthogonally
        ///    rejected] from `fix`. (3)
        /// 2. If `angle` is specified, then `to` is minimally adjusted to have
        ///    the angle `angle` with respect to `from`.
        /// 3. `from` and `to` are normalized.
        /// 4. A rotation is constructed that takes `from` to `to` along the
        ///    shortest path.
        ///
        /// If `from` and `to` are not specified, then a rotation of `angle`
        /// around `fix` is constructed. This method is not recommended because
        /// the direction of the rotation is unspecified and may change
        /// depending on the sign of `fix`.
        ///
        /// </div>
        ///
        /// 1. This is possible with [antigrade] 3 (if its [bulk] is zero) or
        ///    [antigrade] 2 (if its [bulk] is nonzero). For example, in 3D,
        ///    `fix` must be one of the following:
        ///     - A vector (zero [bulk], [grade] 1, [antigrade] 3)
        ///     - A line (nonzero [bulk], [grade] 2, [antigrade] 2)
        /// 2. There are many 180-degree rotations that take any given vector to
        ///    its opposite, so this case is disallowed due to ambiguity.
        /// 3. This results in the component of each vector that is
        ///    perpendicular to `fix`.
        ///
        /// [grade]:
        ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Grade_and_antigrade
        /// [antigrade]:
        ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Grade_and_antigrade
        /// [bulk]:
        ///     https://rigidgeometricalgebra.org/wiki/index.php?title=Bulk_and_weight
        ///
        /// [orthogonally rejected]:
        ///     https://en.wikipedia.org/wiki/Vector_projection
        ///
        /// ```title="Examples of rotation construction"
        /// // 45-degree rotation in the XY plane
        /// rot(from = 'x', to = vec(1, 1, 0))
        ///
        /// // 180-degree rotation around the Z axis
        /// rot(fix = 'z', angle = pi)
        ///
        /// // Jumbling rotation of the Curvy Copter puzzle
        /// rot(fix = vec(1, 1, 0), angle = acos(1/3))
        ///
        /// // 90-degree rotation around an edge of a cube
        /// rot(fix = vec(1, 1, 0), from = 'x', to = 'z')
        /// ```
        #[kwargs(
            fix: Blade = Blade::one(),
            start: Option<Vector>,
            end: Option<Vector>,
            angle: Option<Num>,
        )]
        fn rot(ctx: EvalCtx) -> Motor {
            let ndim = ctx.ndim()?;
            construct_rotation(ndim, fix, start, end, angle).at(ctx.caller_span)?
        }
        fn rot(ctx: EvalCtx, angle: Num) -> Motor {
            if ctx.ndim()? != 2 {
                return Err("`rot(angle)` is only allowed in 2D".at(ctx.caller_span));
            }
            Motor::from_angle_in_axis_plane(0, 1, angle)
        }

        /// Returns the reverse transformation.
        fn rev(transform: Motor) -> Motor {
            transform.reverse()
        }

        /// Returns a transformed object.
        fn transform(transform: Motor, object: Motor) -> Motor {
            transform.transform(&object)
        }
        fn transform(transform: Motor, object: Vector) -> Vector {
            transform.transform(&object)
        }
        fn transform(transform: Motor, object: Point) -> Point {
            transform.transform(&object)
        }

        /// Returns a transformed motor, preserving its orientation.
        fn transform_oriented(transform: Motor, object: Motor) -> Motor {
            let t = transform.transform(&object);
            if transform.is_reflection() {
                t.reverse()
            } else {
                t
            }
        }
    ])
}

fn construct_rotation(
    ndim: u8,
    fix: Blade,
    start: Option<Vector>,
    end: Option<Vector>,
    angle: Option<Float>,
) -> Result<Motor, Cow<'static, str>> {
    let fix = fix.ensure_nonzero_weight();

    let half_angle = angle.map(|a| a / 2.0);
    let sincos_of_half_angle = half_angle.map(|a| (a.sin(), a.cos()));

    let (a, b) = match (start, end) {
        (Some(start), Some(end)) => {
            // IIFE to mimic try_block
            (|| {
                // Reject `start` and `end` start `fix`.
                let start = Blade::from_vector(start)
                    .orthogonal_rejection_from(&fix)?
                    .to_vector()?
                    .normalize()?;
                let end = Blade::from_vector(end)
                    .orthogonal_rejection_from(&fix)?
                    .to_vector()?
                    .normalize()?;

                let a = start.clone();
                let b = match sincos_of_half_angle {
                    Some((sin, cos)) => {
                        let perpendicular = end.rejected_from(&start)?.normalize()?;
                        start * cos + perpendicular * sin
                    }
                    None => (start + end).normalize()?,
                };

                Some((a, b))
            })()
            .ok_or("error constructing rotation (vectors may be zero, or opposite")?
        }

        (None, None) if fix.antigrade(ndim) == Some(2) && !fix.is_zero() && angle.is_some() => {
            let mut dual_basis: [Vector; 2] = fix
                .to_ndim_at_least(ndim)
                .antidual(ndim)
                .ok_or("error taking antidual of `fix`")?
                .ensure_nonzero_weight()
                .basis()
                .try_into()
                .map_err(|e| format!("bad basis for dual of `fix`: {e:?}"))?;

            let pss = Blade::wedge(
                &Blade::wedge(
                    &Blade::from_vector(&dual_basis[0]),
                    &Blade::from_vector(&dual_basis[1]),
                )
                .ok_or("bad basis")?,
                &fix,
            )
            .ok_or("bad basis")?;
            if pss[Axes::antiscalar(ndim)].is_sign_negative() {
                dual_basis.reverse();
            }

            let [start, perpendicular] = dual_basis;

            let (sin, cos) = sincos_of_half_angle.unwrap_or((1.0, 0.0));
            let a = start.clone();
            let b = start * cos + perpendicular * sin;

            (a, b)
        }

        _ => Err("ambiguous rotation")?,
    };

    Ok(Motor::from_normalized_vector_product(a, b))
}

#[cfg(test)]
mod tests {
    use hypermath::assert_approx_eq;

    use super::*;

    #[test]
    fn test_fix_angle_rotation_direction() {
        for ndim in 2..8 {
            println!("Testing {ndim}D");

            let fix = Blade::from_term(Term::unit(
                (2..ndim)
                    .map(|i| Axes::euclidean(i))
                    .fold(Axes::empty(), |a, b| a | b),
            ));
            let t = construct_rotation(ndim, fix, None, None, Some(std::f64::consts::FRAC_PI_2))
                .unwrap();
            assert_approx_eq!(t.transform_vector(vector![1.0]), vector![0.0, 1.0]);

            let fix = Blade::from_term(Term::unit(
                (1..ndim - 1)
                    .map(|i| Axes::euclidean(i))
                    .fold(Axes::empty(), |a, b| a | b),
            ));
            let t = construct_rotation(ndim, fix, None, None, Some(std::f64::consts::FRAC_PI_2))
                .unwrap();
            // cyclic permutation of the basis vectors, so negate depending on
            // parity of the permutation
            let init = Vector::unit(ndim - 1);
            let expected = vector![if ndim % 2 == 1 { 1.0 } else { -1.0 }];
            assert_approx_eq!(t.transform_vector(init), expected);
        }
    }
}

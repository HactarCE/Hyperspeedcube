//! Rhai projective transformation type.

use hypermath::pga::{Axes, Blade, Motor};
use hypermath::{Float, Hyperplane, Point, Vector, VectorRef};
use rhai::Map;

use super::*;

pub fn init_engine(engine: &mut Engine) {
    engine.register_type_with_name::<Motor>("transform");
}

pub fn register(module: &mut Module) {
    new_fn("to_debug").set_into_module(module, |m: &mut Motor| format!("{m:?}"));

    new_fn("==").set_into_module(module, |m1: Motor, m2: Motor| m1.is_equivalent_to(&m2));
    new_fn("!=").set_into_module(module, |m1: Motor, m2: Motor| !m1.is_equivalent_to(&m2));

    new_fn("ident").set_into_module(module, || Motor::ident(0));

    new_fn("refl").set_into_module(module, |ndim: u8| {
        Motor::point_reflection(ndim, &Point::ORIGIN)
    });
    new_fn("refl").set_into_module(module, |_ndim: u8, vector: Vector| {
        Motor::vector_reflection(vector)
    });
    new_fn("refl").set_into_module(module, |ndim: u8, point: Point| {
        Motor::point_reflection(ndim, &point)
    });
    new_fn("refl").set_into_module(module, |ndim: u8, hyperplane: Hyperplane| {
        Motor::plane_reflection(ndim, &hyperplane)
    });

    new_fn("refl").set_into_module(module, |ctx: Ctx<'_>| -> Result<_> {
        let ndim = RhaiState::get_ndim(&ctx)?;
        Ok(Motor::point_reflection(ndim, &Point::ORIGIN))
    });
    new_fn("refl").set_into_module(module, |ctx: Ctx<'_>, vector: Vector| -> Result<_> {
        let _ndim = RhaiState::get_ndim(&ctx)?;
        Ok(Motor::vector_reflection(vector))
    });
    new_fn("refl").set_into_module(module, |ctx: Ctx<'_>, point: Point| -> Result<_> {
        let ndim = RhaiState::get_ndim(&ctx)?;
        Ok(Motor::point_reflection(ndim, &point))
    });
    new_fn("refl").set_into_module(
        module,
        |ctx: Ctx<'_>, hyperplane: Hyperplane| -> Result<_> {
            let ndim = RhaiState::get_ndim(&ctx)?;
            Ok(Motor::plane_reflection(ndim, &hyperplane))
        },
    );

    new_fn("is_refl").set_into_module(module, |m: &mut Motor| m.is_reflection());

    new_fn("rot").set_into_module(module, |ctx: Ctx<'_>, args: Map| -> Result<_> {
        let ndim = RhaiState::get_ndim(&ctx)?;

        let_from_map!(&ctx, args, {
            let fix: OptVecOrSingle<Blade>;
            let from: Option<Vector>;
            let to: Option<Vector>;
            let angle: Option<f64>;
        });

        let fix = fix
            .into_iter()
            .fold(Blade::scalar(1.0), |a, b| Blade::wedge(&a, &b).unwrap_or(a));

        construct_rotation(ndim, fix, from, to, angle)
    });

    new_fn("rev").set_into_module(module, |m: &mut Motor| m.reverse());

    new_fn("*").set_into_module(module, |m1: Motor, m2: Motor| m1 * m2);
}

fn construct_rotation(
    ndim: u8,
    fix: Blade,
    from: Option<Vector>,
    to: Option<Vector>,
    angle: Option<Float>,
) -> Result<Motor> {
    let fix = fix.ensure_nonzero_weight();

    let half_angle = angle.map(|a| a / 2.0);
    let sincos_of_half_angle = half_angle.map(|a| (a.sin(), a.cos()));

    let (a, b) = match (from, to) {
        (Some(from), Some(to)) => {
            // IIFE to mimic try_block
            (|| {
                // Reject `from` and `to` from `fix`.
                let from = Blade::from_vector(from)
                    .orthogonal_rejection_from(&fix)?
                    .to_vector()?
                    .normalize()?;
                let to = Blade::from_vector(to)
                    .orthogonal_rejection_from(&fix)?
                    .to_vector()?
                    .normalize()?;

                let a = from.clone();
                let b = match sincos_of_half_angle {
                    Some((sin, cos)) => {
                        let perpendicular = to.rejected_from(&from)?.normalize()?;
                        from * cos + perpendicular * sin
                    }
                    None => (from + to).normalize()?,
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

            let [from, perpendicular] = dual_basis;

            let (sin, cos) = sincos_of_half_angle.unwrap_or((1.0, 0.0));
            let a = from.clone();
            let b = from * cos + perpendicular * sin;

            (a, b)
        }

        _ => Err("ambiguous rotation")?,
    };

    Ok(Motor::from_normalized_vector_product(a, b))
}

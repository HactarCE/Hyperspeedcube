use eyre::{Result, bail};
use hypermath::pga::{self, Blade, Motor};
use hypermath::{Float, Vector, VectorRef, vector};

use super::*;

/// Lua wrapper for a motor.
#[derive(Debug, Clone)]
pub struct LuaTransform(pub Motor);

impl FromLua for LuaTransform {
    fn from_lua(value: LuaValue, lua: &Lua) -> LuaResult<Self> {
        // TODO: properly handle `pga::Motor` ndim
        let ndim = LuaNdim::get(lua)?;
        cast_userdata(lua, &value).map(|LuaTransform(t)| LuaTransform(t.to_ndim_at_least(ndim)))
    }
}

impl LuaUserData for LuaTransform {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("transform"));

        fields.add_field_method_get("ndim", |_lua, Self(this)| Ok(this.ndim()));
        fields.add_field_method_get("is_ident", |_lua, Self(this)| Ok(this.is_ident()));
        fields.add_field_method_get("is_refl", |_lua, Self(this)| Ok(this.is_reflection()));

        fields.add_field_method_get("rev", |_lua, Self(this)| Ok(Self(this.reverse())));
        fields.add_field_method_get("reverse", |_lua, _| {
            Err::<LuaValue, _>(LuaError::external("use `.rev` instead"))
        });
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, |_lua, Self(this), ()| {
            Ok(format!("transform({this})"))
        });

        // Composition of transforms
        methods.add_meta_method(LuaMetaMethod::Mul, |lua, Self(this), rhs: LuaValue| {
            if let Ok(LuaTransform(rhs)) = <_>::from_lua(rhs.clone(), lua) {
                Ok(Self(this * rhs))
            } else {
                Err(LuaError::external(format!(
                    "cannot multiply transform by {}; use `:transform()` to transform an object",
                    lua_type_name(&rhs),
                )))
            }
        });

        // Exponentiation of transforms
        methods.add_meta_method(LuaMetaMethod::Pow, |_lua, Self(this), rhs: LuaValue| {
            if let LuaValue::Integer(rhs) = rhs {
                Ok(Self(this.powi(rhs)))
            } else if let LuaValue::Number(rhs) = rhs {
                let pow = this.powf(rhs).ok_or(LuaError::external(
                    "error raising transform to non-integer power",
                ))?;
                Ok(Self(pow))
            } else {
                Err(LuaError::external(format!(
                    "cannot raise transform to power of type {}",
                    lua_type_name(&rhs),
                )))
            }
        });

        // Application of transforms
        methods.add_method("transform", |lua, Self(this), obj: Transformable| {
            this.transform(&obj).into_lua(lua).transpose()
        });
        methods.add_method(
            "transform_oriented",
            |_lua, LuaTransform(this), LuaTransform(rhs)| {
                let t = this.transform(&rhs);
                let is_refl = this.is_reflection();
                Ok(LuaTransform(if is_refl { t.reverse() } else { t }))
            },
        );

        // Comparison of transforms
        methods.add_meta_method(LuaMetaMethod::Eq, |_lua, Self(this), Self(other)| {
            Ok(this.is_equivalent_to(&other))
        });
    }
}

impl LuaTransform {
    /// Constructs the identity transformation.
    pub fn construct_identity_lua(lua: &Lua, _: ()) -> LuaResult<Self> {
        Ok(Self(Motor::ident(LuaNdim::get(lua)?)))
    }
    /// Constructs a rotation from a table of values.
    pub fn construct_rotation_lua(lua: &Lua, t: LuaTable) -> LuaResult<Self> {
        let ndim = LuaNdim::get(lua)?;

        let fix: Option<LuaBlade>;
        let from: Option<LuaVector>;
        let to: Option<LuaVector>;
        let angle: Option<Float>;
        unpack_table!(lua.unpack(t {
            fix,
            from,
            to,
            angle,
        }));

        let LuaBlade(fix) = fix.unwrap_or(LuaBlade(Blade::one()));
        let from = from.map(|LuaVector(v)| v);
        let to = to.map(|LuaVector(v)| v);

        Ok(LuaTransform(
            Self::construct_rotation(ndim, fix, from, to, angle)
                .map_err(|e| LuaError::external(format!("{e:#}")))?,
        ))
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
                .ok_or("error constructing rotation (vectors may be zero, or opposite")
                .into_lua_err()?
            }

            (None, None) if fix.antigrade(ndim) == Some(2) && !fix.is_zero() && angle.is_some() => {
                let mut dual_basis: [Vector; 2] = fix
                    .to_ndim_at_least(ndim)
                    .antidual(ndim)
                    .ok_or("error taking antidual of `fix`")
                    .into_lua_err()?
                    .ensure_nonzero_weight()
                    .basis()
                    .try_into()
                    .map_err(|e| format!("bad basis for dual of `fix`: {e:?}"))
                    .into_lua_err()?;

                let pss = Blade::wedge(
                    &Blade::wedge(
                        &Blade::from_vector(&dual_basis[0]),
                        &Blade::from_vector(&dual_basis[1]),
                    )
                    .ok_or_else(|| LuaError::external("bad basis"))?,
                    &fix,
                )
                .ok_or_else(|| LuaError::external("bad basis"))?;
                if pss[pga::Axes::antiscalar(ndim)].is_sign_negative() {
                    dual_basis.reverse();
                }

                let [from, perpendicular] = dual_basis;

                let (sin, cos) = sincos_of_half_angle.unwrap_or((1.0, 0.0));
                let a = from.clone();
                let b = from * cos + perpendicular * sin;

                (a, b)
            }

            _ => bail!("ambiguous rotation"),
        };

        Ok(Motor::from_normalized_vector_product(ndim, a, b))
    }

    /// Constructs a reflection through a vector, across a hyperplane, through a
    /// point, or through the origin.
    pub fn construct_reflection_lua(lua: &Lua, arg: Option<LuaBlade>) -> LuaResult<Self> {
        let ndim = LuaNdim::get(lua)?;

        match arg {
            Some(LuaBlade(b)) => {
                if let Some(point) = b.to_point() {
                    Motor::point_reflection(ndim, point)
                        .ok_or("error constructing point reflection")
                } else if let Some(vector) = b.to_vector() {
                    Motor::vector_reflection(ndim, vector)
                        .ok_or("cannot reflect through zero vector")
                } else if let Some(hyperplane) = b.to_hyperplane(ndim) {
                    Motor::plane_reflection(ndim, &hyperplane)
                        .ok_or("error constructing plane reflection")
                } else {
                    return Err(LuaError::FromLuaConversionError {
                        from: "blade",
                        to: "point, vector, or hyperplane".to_owned(),
                        message: None,
                    });
                }
            }
            None => Motor::point_reflection(ndim, vector![])
                .ok_or("error constructing point reflection"), // reflect through origin
        }
        .map(Self)
        .into_lua_err()
    }
}

#[cfg(test)]
mod tests {
    use hypermath::assert_approx_eq;

    use super::*;

    #[test]
    fn test_fix_angle_rotation_direction() {
        for ndim in 2..8 {
            println!("Testing {ndim}D");

            let fix = Blade::from_term(pga::Term::unit(
                (2..ndim)
                    .map(|i| pga::Axes::euclidean(i))
                    .fold(pga::Axes::empty(), |a, b| a | b),
            ));
            let t = LuaTransform::construct_rotation(
                ndim,
                fix,
                None,
                None,
                Some(std::f64::consts::FRAC_PI_2),
            )
            .unwrap();
            assert_approx_eq!(t.transform_vector(vector![1.0]), vector![0.0, 1.0]);

            let fix = Blade::from_term(pga::Term::unit(
                (1..ndim - 1)
                    .map(|i| pga::Axes::euclidean(i))
                    .fold(pga::Axes::empty(), |a, b| a | b),
            ));
            let t = LuaTransform::construct_rotation(
                ndim,
                fix,
                None,
                None,
                Some(std::f64::consts::FRAC_PI_2),
            )
            .unwrap();
            // cyclic permutation of the basis vectors, so negate depending on
            // parity of the permutation
            let init = Vector::unit(ndim - 1);
            let expected = vector![if ndim % 2 == 1 { 1.0 } else { -1.0 }];
            assert_approx_eq!(t.transform_vector(init), expected);
        }
    }
}

use hypermath::pga::{Blade, Motor};
use hypermath::{vector, Float, VectorRef};

use super::*;

/// Lua wrapper for a motor.
#[derive(Debug, Clone)]
pub struct LuaTransform(pub Motor);

impl<'lua> FromLua<'lua> for LuaTransform {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        // TODO: properly handle `pga::Motor` ndim
        let ndim = LuaNdim::get(lua)?;
        cast_userdata(lua, &value).map(|LuaTransform(t)| LuaTransform(t.to_ndim_at_least(ndim)))
    }
}

impl LuaUserData for LuaTransform {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("transform"));

        fields.add_field_method_get("ndim", |_lua, Self(this)| Ok(this.ndim()));
        fields.add_field_method_get("is_ident", |_lua, Self(this)| Ok(this.is_ident()));
        fields.add_field_method_get("is_refl", |_lua, Self(this)| Ok(this.is_reflection()));

        fields.add_field_method_get("rev", |_lua, Self(this)| Ok(Self(this.reverse())));
        fields.add_field_method_get("reverse", |_lua, _| {
            Err::<LuaValue<'_>, _>(LuaError::external("use `.rev` instead"))
        });
    }

    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, |_lua, Self(this), ()| {
            Ok(format!("transform({this})"))
        });

        // Composition of transforms
        methods.add_meta_method(LuaMetaMethod::Mul, |lua, Self(this), rhs: LuaValue<'_>| {
            if let Ok(LuaTransform(rhs)) = <_>::from_lua(rhs.clone(), lua) {
                Ok(Self(this * rhs))
            } else {
                Err(LuaError::external(format!(
                    "cannot multiply transform by {}; use `:transform()` to transform an object",
                    lua_type_name(&rhs),
                )))
            }
        });

        // Application of transforms
        methods.add_method("transform", |lua, Self(this), obj: Transformable| {
            this.transform(&obj).into_lua(lua).transpose()
        });
        // TODO: clean this up and pick a name, dammit
        fn transform_oriented(
            _lua: &Lua,
            LuaTransform(this): &LuaTransform,
            LuaTransform(rhs): LuaTransform,
        ) -> LuaResult<LuaTransform> {
            let t = this.transform(&rhs);
            let is_refl = this.is_reflection();
            Ok(LuaTransform(if is_refl { t.reverse() } else { t }))
        }
        methods.add_method("transform_oriented", transform_oriented);
        methods.add_method("transform_keep_orientation", transform_oriented);
        methods.add_method("tfko", transform_oriented);

        // Comparison of transforms
        methods.add_meta_method(LuaMetaMethod::Eq, |_lua, Self(this), Self(other)| {
            Ok(this.is_equivalent_to(&other))
        });
    }
}

impl LuaTransform {
    /// Constructs the identity transformation.
    pub fn construct_identity(lua: &Lua, _: ()) -> LuaResult<Self> {
        Ok(Self(Motor::ident(LuaNdim::get(lua)?)))
    }
    /// Constructs a rotation from a table of values.
    pub fn construct_rotation(lua: &Lua, t: LuaTable<'_>) -> LuaResult<Self> {
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

        let LuaBlade(fix) = fix.unwrap_or(LuaBlade(Blade::one(ndim)));
        let fix = fix.ensure_nonzero_weight();

        let half_angle = angle.map(|a| a / 2.0);
        let sincos_of_half_angle = half_angle.map(|a| (a.sin(), a.cos()));

        let (a, b) = match (from, to) {
            (Some(LuaVector(from)), Some(LuaVector(to))) => {
                // IIFE to mimic try_block
                (|| {
                    // Reject `from` and `to` from `fix`.
                    let from = Blade::from_vector(ndim, from)
                        .orthogonal_rejection_from(&fix)?
                        .to_vector()?
                        .normalize()?;
                    let to = Blade::from_vector(ndim, to)
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

            (None, None) if fix.antigrade() == 2 && !fix.is_zero() && angle.is_some() => {
                let [from, perpendicular] = fix
                    .antidual()
                    .ensure_nonzero_weight()
                    .basis()
                    .try_into()
                    .map_err(|e| format!("bad basis for dual of `fix`: {e:?}"))
                    .into_lua_err()?;

                let (sin, cos) = sincos_of_half_angle.unwrap_or((1.0, 0.0));
                let a = from.clone();
                let b = from * cos + perpendicular * sin;

                (a, b)
            }

            _ => return Err(LuaError::external("ambiguous rotation")),
        };

        Ok(LuaTransform(Motor::from_normalized_vector_product(
            ndim, a, b,
        )))
    }
    /// Constructs a reflection through a vector, across a hyperplane, through a
    /// point, or through the origin.
    pub fn construct_reflection(lua: &Lua, arg: Option<LuaBlade>) -> LuaResult<Self> {
        let ndim = LuaNdim::get(lua)?;

        Ok(Self(match arg {
            Some(LuaBlade(b)) => {
                if let Some(point) = b.to_point() {
                    Motor::point_reflection(ndim, point)
                } else if let Some(vector) = b.to_vector() {
                    Motor::vector_reflection(ndim, vector)
                        .ok_or("cannot reflect through zero vector")
                        .into_lua_err()?
                } else if let Some(hyperplane) = b.to_hyperplane() {
                    Motor::plane_reflection(ndim, &hyperplane)
                } else {
                    return Err(LuaError::FromLuaConversionError {
                        from: "blade",
                        to: "point, vector, or hyperplane",
                        message: None,
                    });
                }
            }
            None => Motor::point_reflection(ndim, vector![]), // reflect through origin
        }))
    }
}

use hypermath::pga::{Blade, Motor};
use hypermath::vector;

use super::*;

/// Lua wrapper for a motor.
#[derive(Debug, Clone)]
pub struct LuaTransform(pub Motor);

impl<'lua> FromLua<'lua> for LuaTransform {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        cast_userdata(lua, &value)
    }
}

impl LuaUserData for LuaTransform {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("transform"));

        fields.add_field_method_get("reverse", |_lua, Self(this)| Ok(Self(this.reverse())));
        fields.add_field_method_get("rev", |_lua, Self(this)| Ok(Self(this.reverse())));
    }

    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, |_lua, Self(this), ()| {
            Ok(format!("transform({this})"))
        });

        // Composition of transforms
        methods.add_meta_method(LuaMetaMethod::Mul, |_lua, Self(this), Self(rhs)| {
            Ok(Self(this * rhs))
        });

        methods.add_method("ndim", |_lua, Self(this), ()| Ok(this.ndim()));

        // Application of transforms
        methods.add_method("transform", |lua, Self(this), obj: Transformable| {
            this.transform(&obj).into_lua(lua).transpose()
        });
    }
}

impl LuaTransform {
    /// Constructs a rotation from a table of values.
    pub fn construct_rotation(lua: &Lua, t: LuaTable<'_>) -> LuaResult<Self> {
        let ndim = LuaNdim::get(lua)?;

        // TODO: allow fixing multiple vectors using blades
        let fix: Option<LuaBlade>;
        let from: LuaVector;
        let to: LuaVector;
        unpack_table!(lua.unpack(t { fix, from, to }));

        let LuaBlade(fix) = fix.unwrap_or(LuaBlade(Blade::one(ndim)));
        let LuaVector(from) = from;
        let LuaVector(to) = to;

        let from = Blade::from_vector(ndim, from);
        let to = Blade::from_vector(ndim, to);

        // Reject `from` and `to` from `fix`.
        let from = from
            .orthogonal_rejection_from(&fix)
            .and_then(|b| b.to_vector())
            .unwrap_or(vector![]);
        let to = to
            .orthogonal_rejection_from(&fix)
            .and_then(|b| b.to_vector())
            .unwrap_or(vector![]);

        let rot = Motor::rotation(ndim, from, to)
            .ok_or("error constructing rotation (vectors may be zero, or opposite")
            .into_lua_err()?;

        Ok(LuaTransform(rot))
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

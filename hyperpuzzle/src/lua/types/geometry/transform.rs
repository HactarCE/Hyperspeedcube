use hypermath::{Blade, Isometry, Multivector};

use super::*;

#[derive(Debug, Default, Clone)]
pub struct LuaTransform(pub Isometry);

impl<'lua> FromLua<'lua> for LuaTransform {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        cast_userdata(lua, &value)
    }
}

impl LuaTransform {
    pub fn construct_rotation(lua: &Lua, t: LuaTable<'_>) -> LuaResult<Self> {
        let fix: Option<LuaMultivector>;
        let from: LuaVector;
        let to: LuaVector;
        unpack_table!(lua.unpack(t { fix, from, to }));

        let fix = match fix {
            Some(LuaMultivector(m)) => m,
            None => Multivector::scalar(1.0),
        };
        let fix = Blade::try_from(fix)
            .into_lua_err()
            .context("`fix` must be a blade (no mixed grade)")?;
        let fix_inv = fix
            .inverse()
            .ok_or_else(|| LuaError::external("`fix` must be an invertible blade"))?;

        // Reject `from` and `to` from `fix`.
        let from = (&fix_inv << (Blade::vector(from.0) ^ &fix)).to_vector();
        let to = (&fix_inv << (Blade::vector(to.0) ^ &fix)).to_vector();

        let rot = Isometry::from_vec_to_vec(from, to).ok_or_else(|| {
            LuaError::external("error constructing rotation (vectors may be zero, or opposite")
        })?;

        Ok(LuaTransform(rot))
    }
    pub fn construct_reflection(lua: &Lua, args: LuaMultiValue<'_>) -> LuaResult<Self> {
        if args.is_empty() {
            return Err(LuaError::external("at least one vector is required"));
        }
        args.into_iter()
            .map(|value| LuaVector::from_lua(value, lua))
            .try_fold(Isometry::default(), |t, vector| {
                let LuaVector(v) = vector?;
                let refl = Isometry::from_reflection(v)
                    .ok_or_else(|| LuaError::external("cannot reflect through zero vector"))?;
                Ok(t * refl)
            })
            .map(LuaTransform)
    }
}

impl LuaUserData for LuaTransform {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("transform"));
    }

    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, |_lua, Self(this), ()| {
            Ok(format!("transform({this})"))
        });

        methods.add_method("ndim", |lua, Self(this), ()| Ok(this.ndim()));

        methods.add_meta_method(LuaMetaMethod::Mul, |lua, Self(this), rhs| {
            Transformable::from_lua(rhs, lua)?
                .transform(this)?
                .into_lua(lua)
                .transpose()
        });
    }
}

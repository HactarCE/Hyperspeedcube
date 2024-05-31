use hypermath::prelude::*;

use super::*;

/// Lua wrapper for a set of hyperplanes.
#[derive(Debug, Clone)]
pub struct LuaHyperplaneSet(pub Vec<Hyperplane>);

impl<'lua> FromLua<'lua> for LuaHyperplaneSet {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        if let Ok(LuaHyperplane(h)) = lua.unpack(value.clone()) {
            Ok(Self(vec![h]))
        } else if let Ok(LuaSequence(surfaces)) = lua.unpack(value.clone()) {
            Ok(Self(
                surfaces.into_iter().map(|LuaHyperplane(m)| m).collect(),
            ))
        } else {
            lua_convert_err(&value, "hyperplane or table of hyperplanes")
        }
    }
}

/// Lua conversion wrapper for a hyperplane.
///
/// This is not actually a Lua type since it does not implement [`LuaUserData`].
#[derive(Debug, Clone)]
pub struct LuaHyperplane(pub Hyperplane);

impl<'lua> FromLua<'lua> for LuaHyperplane {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        if let Ok(LuaVector(v)) = lua.unpack(value.clone()) {
            Ok(Self(
                Hyperplane::from_pole(v)
                    .ok_or("plane pole cannot be zero")
                    .into_lua_err()?,
            ))
        } else if let Ok(LuaBlade(b)) = cast_userdata(lua, &value) {
            match b.to_hyperplane() {
                Some(h) => Ok(Self(h)),
                None => lua_convert_err(&value, "hyperplane"),
            }
        } else if let LuaValue::Table(t) = value {
            Self::construct_from_table(lua, t)
        } else {
            lua_convert_err(
                &value,
                "hyperplane, blade, vector, or table describing a hyperplane",
            )
        }
    }
}

impl<'lua> IntoLua<'lua> for LuaHyperplane {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
        LuaBlade(pga::Blade::from_hyperplane(LuaNdim::get(lua)?, &self.0)).into_lua(lua)
    }
}

impl LuaTypeName for LuaHyperplane {
    fn type_name(_lua: &Lua) -> LuaResult<&'static str> {
        Ok("hyperplane")
    }
}

impl LuaHyperplane {
    /// Constructs a plane from a table of values.
    pub fn construct_from_table(lua: &Lua, t: LuaTable<'_>) -> LuaResult<Self> {
        let arg_count = t.clone().pairs::<LuaValue<'_>, LuaValue<'_>>().count();
        let ensure_args_len = |n| {
            if n == arg_count {
                Ok(())
            } else {
                Err(LuaError::external(
                    "bad hyperplane construction; too many keys",
                ))
            }
        };

        let distance: Option<Float>;
        let normal: Option<LuaVector>;
        let point: Option<LuaPoint>;
        let pole: Option<LuaVector>;

        unpack_table!(lua.unpack(t {
            distance,
            normal,
            point,
            pole
        }));

        if let Some(LuaVector(pole)) = pole {
            ensure_args_len(1)?;
            Hyperplane::from_pole(pole).ok_or("plane pole cannot be zero")
        } else if let Some(LuaVector(normal)) = normal {
            ensure_args_len(2)?;
            if let Some(distance) = distance {
                Hyperplane::new(normal, distance).ok_or("plane normal vector cannot be zero")
            } else if let Some(LuaPoint(point)) = point {
                Hyperplane::through_point(normal, point).ok_or("plane normal vector cannot be zero")
            } else {
                Err("either `distance` or `point` must be specified with `normal`")
            }
        } else {
            Err("bad plane construction; \
                 expected keys such as `distance`, \
                 `normal`, `point`, and `pole`")
        }
        .map(Self)
        .into_lua_err()
    }

    /// Returns a blade representing the hyperplane.
    pub fn to_blade(&self, lua: &Lua) -> LuaResult<LuaBlade> {
        let ndim = LuaNdim::get(lua)?;
        Ok(LuaBlade(pga::Blade::from_hyperplane(ndim, &self.0)))
    }
}

impl TransformByMotor for LuaHyperplane {
    fn transform_by(&self, m: &hypermath::pga::Motor) -> Self {
        Self(self.0.transform_by(m))
    }
}

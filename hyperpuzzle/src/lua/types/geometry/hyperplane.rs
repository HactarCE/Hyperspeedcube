use hypermath::prelude::*;

use super::*;

/// Lua wrapper for a set of hyperplanes.
#[derive(Debug, Default, Clone)]
pub struct LuaHyperplaneSet(pub Vec<Hyperplane>);

impl FromLua for LuaHyperplaneSet {
    fn from_lua(value: LuaValue, lua: &Lua) -> LuaResult<Self> {
        if value.is_nil() {
            Ok(Self(vec![]))
        } else if let Ok(LuaHyperplane(h)) = lua.unpack(value.clone()) {
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

/// Lua conversion wrapper for constructing a hyperplane from a multivalue.
#[derive(Debug, Clone)]
pub struct LuaHyperplaneFromMultivalue(pub Hyperplane);

impl FromLuaMulti for LuaHyperplaneFromMultivalue {
    fn from_lua_multi(values: LuaMultiValue, lua: &Lua) -> LuaResult<Self> {
        let hyperplane = if values.len() == 2 {
            let LuaVector(normal) = <_>::from_lua(values.get(0).unwrap_or(&LuaNil).clone(), lua)?;
            let distance: Float = <_>::from_lua(values.get(1).unwrap_or(&LuaNil).clone(), lua)?;
            Hyperplane::new(normal, distance)
                .ok_or("plane normal vector cannot be zero")
                .into_lua_err()?
        } else {
            LuaHyperplane::from_lua_multi(values, lua)?.0
        };

        Ok(Self(hyperplane))
    }
}

/// Lua conversion wrapper for a hyperplane.
///
/// This is not actually a Lua type since it does not implement [`LuaUserData`].
#[derive(Debug, Clone)]
pub struct LuaHyperplane(pub Hyperplane);

impl FromLua for LuaHyperplane {
    fn from_lua(value: LuaValue, lua: &Lua) -> LuaResult<Self> {
        if let Ok(this) = cast_userdata(lua, &value) {
            Ok(this)
        } else if let Ok(LuaVector(v)) = lua.unpack(value.clone()) {
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

impl LuaUserData for LuaHyperplane {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("hyperplane"));

        fields.add_field_method_get("ndim", |_lua, Self(this)| Ok(this.normal().ndim()));
        fields.add_field_method_get("flip", |_lua, Self(this)| Ok(Self(this.flip())));
        fields.add_field_method_get("normal", |_lua, Self(this)| {
            Ok(LuaVector(this.normal().clone()))
        });
        fields.add_field_method_get("distance", |_lua, Self(this)| Ok(this.distance()));
        fields.add_field_method_get("blade", |lua, this| this.to_blade(lua));

        fields.add_field_method_get("region", |_lua, Self(this)| {
            Ok(LuaRegion::HalfSpace(this.clone()))
        });
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("signed_distance", |_lua, Self(this), LuaPoint(p)| {
            Ok(this.signed_distance_to_point(p))
        });

        methods.add_meta_method(LuaMetaMethod::Eq, |_lua, Self(this), Self(other)| {
            Ok(approx_eq(this, &other))
        });
    }
}

impl LuaHyperplane {
    /// Constructs a plane from a table of values.
    pub fn construct_from_table(lua: &Lua, t: LuaTable) -> LuaResult<Self> {
        let arg_count = t.clone().pairs::<LuaValue, LuaValue>().count();
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

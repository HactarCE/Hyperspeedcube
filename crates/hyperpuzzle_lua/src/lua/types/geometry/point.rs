use hypermath::prelude::*;

use super::*;

/// Lua conversion wrapper for constructing a point from a multivalue.
pub struct LuaPointFromMultiValue(pub Vector);

impl FromLuaMulti for LuaPointFromMultiValue {
    fn from_lua_multi(values: LuaMultiValue, lua: &Lua) -> LuaResult<Self> {
        if let Ok(LuaPoint(p)) = lua.unpack_multi(values.clone()) {
            Ok(Self(p))
        } else {
            lua.unpack_multi(values)
                .map(|LuaVectorFromMultiValue(v)| Self(v))
        }
    }
}

/// Lua conversion wrapper for constructing a point from a single value, which
/// may be a blade representing a vector.
pub struct LuaPoint(pub Vector);

impl FromLua for LuaPoint {
    fn from_lua(value: LuaValue, lua: &Lua) -> LuaResult<Self> {
        if let Ok(LuaBlade(b)) = cast_userdata(lua, &value) {
            match b.to_point().or_else(|| b.to_vector()) {
                Some(v) => Ok(Self(v)),
                None => Err(LuaError::FromLuaConversionError {
                    from: "blade",
                    to: "point".to_owned(),
                    message: Some(format!("expected 1-blade; got {}-blade", b.grade())),
                }),
            }
        } else {
            lua.unpack(value).map(|LuaVector(v)| Self(v))
        }
    }
}

impl IntoLua for LuaPoint {
    fn into_lua(self, lua: &Lua) -> LuaResult<LuaValue> {
        LuaBlade::from_point(self.0)?.into_lua(lua)
    }
}

impl TransformByMotor for LuaPoint {
    fn transform_by(&self, m: &pga::Motor) -> Self {
        Self(m.transform_point(&self.0))
    }
}

use hypermath::collections::approx_hashmap::FloatHash;
use hypermath::prelude::*;
use itertools::Itertools;

use super::*;

/// Lua conversion wrapper for constructing a vector from a multivalue.
#[derive(Debug, Clone)]
pub struct LuaVectorFromMultiValue(pub Vector);

impl FromLuaMulti for LuaVectorFromMultiValue {
    #[allow(clippy::get_first)]
    fn from_lua_multi(values: LuaMultiValue, lua: &Lua) -> LuaResult<Self> {
        match values.get(0) {
            None => Ok(Self(vector![])),
            Some(v) if v.is_number() || v.is_integer() => values
                .into_iter()
                .map(|v| lua.unpack(v).map(|LuaNumberNoConvert(x)| x as Float))
                .try_collect()
                .map(Self),
            Some(_) if values.len() > 1 => Err(LuaError::FromLuaConversionError {
                from: "values",
                to: "numbers".to_owned(),
                message: None,
            }),
            Some(v) => match lua.unpack(v.clone()) {
                Ok(LuaVector(v)) => Ok(Self(v)),
                Err(_) => lua_convert_err(v, "number, vector, multivector, table, or axis name"),
            },
        }
    }
}

/// Lua conversion wrapper for constructing a vector from a single value.
#[derive(Debug, Clone)]
pub struct LuaVector(pub Vector);

impl FromLua for LuaVector {
    fn from_lua(value: LuaValue, lua: &Lua) -> LuaResult<Self> {
        match value {
            LuaValue::Table(t) => Self::construct_from_table(t),
            LuaValue::String(s) => {
                let LuaVectorIndex(axis) = s.to_string_lossy().parse().into_lua_err()?;
                Ok(Self(Vector::unit(axis)))
            }
            v => {
                if let Ok(LuaBlade(b)) = cast_userdata(lua, &v) {
                    match b.to_point().or_else(|| b.to_vector()) {
                        Some(v) => Ok(Self(v)),
                        None => Err(LuaError::FromLuaConversionError {
                            from: "blade",
                            to: "vector".to_owned(),
                            message: format!("expected 1-blade; got {}-blade", b.grade()).into(),
                        }),
                    }
                } else if let Ok(axis) = cast_userdata::<LuaAxis>(lua, &v) {
                    Ok(Self(axis.vector()?))
                } else {
                    lua_convert_err(&v, "vector, point, table, or axis name")
                }
            }
        }
    }
}

impl IntoLua for LuaVector {
    fn into_lua(self, lua: &Lua) -> LuaResult<LuaValue> {
        let ndim = LuaNdim::get(lua)?;
        LuaBlade(pga::Blade::from_vector(ndim, self.0)).into_lua(lua)
    }
}

impl LuaTypeName for LuaVector {
    fn type_name(_lua: &Lua) -> LuaResult<&'static str> {
        Ok("vector")
    }
}

impl LuaVector {
    /// Constructs a vector from a table of values.
    pub fn construct_from_table(t: LuaTable) -> LuaResult<Self> {
        let mut ret = vector![];
        for pair in t.pairs() {
            let (LuaVectorIndex(k), v): (_, Float) = pair?;
            ret = ret.pad(k + 1);
            ret[k] = v;
        }
        Ok(LuaVector(ret))
    }
}

impl TransformByMotor for LuaVector {
    fn transform_by(&self, m: &pga::Motor) -> Self {
        Self(m.transform_vector(&self.0))
    }
}

impl ApproxHashMapKey for LuaVector {
    type Hash = <Vector as ApproxHashMapKey>::Hash;

    fn approx_hash(&self, float_hash_fn: impl FnMut(Float) -> FloatHash) -> Self::Hash {
        self.0.approx_hash(float_hash_fn)
    }
}

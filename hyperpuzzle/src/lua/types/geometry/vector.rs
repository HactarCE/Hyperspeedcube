use hypermath::{collections::approx_hashmap::FloatHash, prelude::*};
use itertools::Itertools;

use super::*;

/// Lua conversion wrapper for constructing a vector from a multivalue.
#[derive(Debug, Clone)]
pub struct LuaVectorFromMultiValue(pub Vector);

impl<'lua> FromLuaMulti<'lua> for LuaVectorFromMultiValue {
    fn from_lua_multi(values: LuaMultiValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        match values.get(0) {
            None => return Ok(Self(vector![])),
            Some(v) if v.is_number() || v.is_integer() => values
                .into_iter()
                .map(|v| lua.unpack(v).map(|LuaNumberNoConvert(x)| x as Float))
                .try_collect()
                .map(Self),
            Some(_) if values.len() > 1 => Err(LuaError::FromLuaConversionError {
                from: "values",
                to: "numbers",
                message: None,
            }),
            Some(v) => match lua.unpack(v.clone()) {
                Ok(LuaVector(v)) => Ok(Self(v)),
                Err(_) => lua_convert_err(&v, "number, vector, multivector, table, or axis name"),
            },
        }
    }
}

/// Lua conversion wrapper for constructing a vector from a single value.
#[derive(Debug, Clone)]
pub struct LuaVector(pub Vector);

impl<'lua> FromLua<'lua> for LuaVector {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        match value {
            LuaNil => Ok(Self(vector![])),
            LuaValue::Table(t) => Self::construct_from_table(t),
            LuaValue::String(s) => {
                let LuaVectorIndex(axis) = s.to_string_lossy().parse().into_lua_err()?;
                Ok(Self(Vector::unit(axis)))
            }
            v => {
                if let Ok(LuaBlade(b)) = cast_userdata(lua, &v) {
                    match b.to_vector().or_else(|| b.to_point()) {
                        Some(v) => Ok(Self(v)),
                        None => Err(LuaError::FromLuaConversionError {
                            from: "blade",
                            to: "vector",
                            message: format!("expected 1-blade; got {}-blade", b.grade()).into(),
                        }),
                    }
                } else if let Ok(axis) = cast_userdata::<LuaAxis>(lua, &v) {
                    Ok(Self(axis.vector()?.into()))
                } else {
                    lua_convert_err(&v, "vector, multivector, table, or axis name")
                }
            }
        }
    }
}

impl<'lua> IntoLua<'lua> for LuaVector {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
        let ndim = LuaNdim::get(lua)?;
        LuaBlade(pga::Blade::from_vector(ndim, self.0)).into_lua(lua)
    }
}

impl LuaVector {
    /// Constructs a vector from a table of values.
    pub fn construct_from_table(t: LuaTable<'_>) -> LuaResult<Self> {
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

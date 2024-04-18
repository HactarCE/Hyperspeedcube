use hypermath::prelude::*;
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

/// Lua wrapper for a vector.
#[derive(Debug, Clone)]
pub struct LuaVector(pub Vector);

impl<'lua> FromLua<'lua> for LuaVector {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        match value {
            LuaNil => Ok(LuaVector(vector![])),
            LuaValue::Table(t) => LuaVector::construct_from_table(t),
            LuaValue::String(s) => {
                let LuaVectorIndex(axis) = s.to_string_lossy().parse().into_lua_err()?;
                Ok(LuaVector(Vector::unit(axis)))
            }
            v => {
                if let Ok(LuaVector(v)) = cast_userdata(lua, &v) {
                    Ok(LuaVector(v))
                } else if let Ok(LuaMultivector(m)) = cast_userdata(lua, &v) {
                    Ok(LuaVector(m.grade_project(1).to_vector()))
                } else if let Ok(axis) = cast_userdata::<LuaAxis>(lua, &v) {
                    Ok(Self(axis.vector()?.into()))
                } else {
                    lua_convert_err(&v, "vector, multivector, table, or axis name")
                }
            }
        }
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

    /// Transforms the vector by `t`.
    pub fn transform(&self, t: &Isometry) -> Self {
        Self(t.transform_vector(&self.0))
    }
}

impl LuaUserData for LuaVector {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("vector"));
    }

    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, |_lua, Self(this), ()| {
            Ok(format!("vec({})", this.iter().join(", ")))
        });

        methods.add_method("ndim", |_lua, Self(this), ()| Ok(this.ndim()));
        methods.add_method("mag2", |_lua, Self(this), ()| Ok(this.mag2()));
        methods.add_method("mag", |_lua, Self(this), ()| Ok(this.mag()));

        methods.add_method("at_ndim", |_lua, Self(this), LuaNdim(new_ndim)| {
            let mut ret = this.clone();
            ret.resize(new_ndim);
            Ok(LuaVector(ret))
        });

        methods.add_method("normalized", |_lua, Self(this), new_mag: Option<Float>| {
            let new_mag = new_mag.unwrap_or(1.0);
            let ret = match hypermath::util::try_div(new_mag, this.mag()) {
                Some(scale) => this * scale,
                None => this.clone(),
            };
            Ok(LuaVector(ret))
        });

        methods.add_method("dot", |_lua, Self(this), LuaVector(other)| {
            Ok(this.dot(other))
        });
        methods.add_method("cross", |_lua, Self(this), LuaVector(other)| {
            Ok(LuaVector(vector![
                this[1] * other[2] - this[2] * other[1],
                this[2] * other[0] - this[0] * other[2],
                this[0] * other[1] - this[1] * other[0],
            ]))
        });

        methods.add_method(
            "projected_to",
            |_lua, Self(this), LuaVector(other)| match this.projected_to(&other) {
                Some(result) => Ok(LuaVector(result)),
                None => Err(LuaError::external("cannot project to zero vector")),
            },
        );
        methods.add_method(
            "rejected_from",
            |_lua, Self(this), LuaVector(other)| match this.rejected_from(&other) {
                Some(result) => Ok(LuaVector(result)),
                None => Err(LuaError::external("cannot reject from zero vector")),
            },
        );

        // Vector + Vector
        methods.add_meta_function(
            LuaMetaMethod::Add,
            |_lua, (LuaVector(lhs), LuaVector(rhs))| Ok(LuaVector(lhs + rhs)),
        );

        // Vector - Vector
        methods.add_meta_function(
            LuaMetaMethod::Sub,
            |_lua, (LuaVector(lhs), LuaVector(rhs))| Ok(LuaVector(lhs - rhs)),
        );

        // Vector * f64; f64 * Vector
        methods.add_meta_function(LuaMetaMethod::Mul, |lua, args: LuaMultiValue<'_>| {
            if let Ok((LuaVector(v), a)) = lua.unpack_multi(args.clone()) {
                let a: Float = a;
                Ok(LuaVector(v * a))
            } else if let Ok((a, LuaVector(v))) = lua.unpack_multi(args.clone()) {
                let a: Float = a;
                Ok(LuaVector(v * a))
            } else {
                let [a, b]: [LuaValue<'_>; 2] = lua.unpack_multi(args)?;
                Err(LuaError::external(format!(
                    "cannot multiply {} and {}",
                    a.type_name(),
                    b.type_name(),
                )))
            }
        });

        // Vector / f64
        methods.add_meta_function(
            LuaMetaMethod::Div,
            |_lua, (LuaVector(lhs), rhs): (_, Float)| Ok(LuaVector(lhs / rhs)),
        );

        // -Vector
        methods.add_meta_function(LuaMetaMethod::Unm, |_lua, LuaVector(v)| Ok(LuaVector(-v)));

        // Vector ^ Vector
        methods.add_meta_function(
            LuaMetaMethod::Pow,
            |_lua, (LuaMultivector(lhs), LuaMultivector(rhs))| Ok(LuaMultivector(lhs ^ rhs)),
        );

        // Vector == Vector
        methods.add_meta_function(LuaMetaMethod::Eq, |_lua, (LuaVector(a), LuaVector(b))| {
            Ok(approx_eq(&a, &b))
        });

        // Vector[index]
        methods.add_meta_method(
            LuaMetaMethod::Index,
            |lua, Self(this), arg: LuaValue<'_>| match lua.unpack(arg) {
                Ok(LuaVectorIndex(index)) => Ok(Some(this.get(index))),
                Err(_) => Ok(None),
            },
        );

        // We do not support `LuaMetaMethod::NewIndex` because this can be used
        // to mutate aliased vectors, which is very confusing.
        methods.add_meta_method(
            LuaMetaMethod::NewIndex,
            |_lua, Self(_), _: LuaMultiValue<'_>| -> LuaResult<()> {
                Err(LuaError::external(
                    "mutation of vectors is not allowed. \
                     construct a new vector instead.",
                ))
            },
        );

        // #Vector
        methods.add_meta_method(LuaMetaMethod::Len, |_lua, Self(this), ()| Ok(this.ndim()));

        // tostring(Vector)
        methods.add_meta_method(LuaMetaMethod::ToString, |_lua, Self(this), ()| {
            Ok(this.to_string())
        });

        // pairs(Vector)
        methods.add_meta_method(LuaMetaMethod::Pairs, |lua, this, ()| {
            let vector_iter = lua.create_function(|_lua, (LuaVector(v), LuaVectorIndex(i))| {
                if i < v.ndim() {
                    // Add 2 because `LuaVectorIndex` subtracts 1 to be zero-indexed
                    Ok((Some(i + 2), Some(v[i])))
                } else {
                    Ok((None, None))
                }
            })?;

            Ok((vector_iter, this.clone(), 0))
        });
    }
}

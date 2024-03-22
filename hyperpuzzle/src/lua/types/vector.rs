use hypermath::prelude::*;
use itertools::Itertools;

use super::*;

lua_userdata_value_conversion_wrapper! {
    #[name = "vector", convert_str = "vector, table, or axis string"]
    pub struct LuaVector(Vector) = |_lua| {
        <LuaTable<'_>>(t)  => Ok(LuaVector::construct_from_table(t)?),
        <LuaAxisName>(axis) => Ok(Vector::unit(axis.0)),
    }
}
lua_userdata_multivalue_conversion_wrapper!(pub struct LuaConstructVector(Vector) = LuaVector::construct_unwrapped_from_multivalue);

impl LuaVector {
    fn construct_from_table(t: LuaTable<'_>) -> LuaResult<Vector> {
        let mut ret = vector![];
        for pair in t.pairs() {
            let (LuaVectorIndex(k), v): (_, Float) = pair?;
            ret = ret.pad(k + 1);
            ret[k] = v;
        }
        Ok(ret)
    }

    fn construct_unwrapped_from_multivalue<'lua>(
        lua: LuaContext<'lua>,
        values: LuaMultiValue<'lua>,
    ) -> LuaResult<Vector> {
        if let Ok(t) = LuaTable::from_lua_multi(values.clone(), lua) {
            return Self::construct_from_table(t);
        }

        if let Ok(LuaAxisName(i)) = <_>::from_lua_multi(values.clone(), lua) {
            return Ok(Vector::unit(i));
        }

        values
            .into_iter()
            .map(|v| Float::from_lua(v, lua))
            .try_collect()
            .map_err(|_| {
                LuaError::external("expected vector, table, axis name, or sequence of numbers")
            })
    }

    pub fn construct_from_multivalue<'lua>(
        lua: LuaContext<'lua>,
        values: LuaMultiValue<'lua>,
    ) -> LuaResult<LuaVector> {
        Self::construct_unwrapped_from_multivalue(lua, values).map(Self)
    }
}

impl LuaUserData for LuaNamedUserData<Vector> {
    fn add_methods<'lua, T: LuaUserDataMethods<'lua, Self>>(methods: &mut T) {
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

        methods.add_method("dot", |_lua, Self(this), LuaConstructVector(other)| {
            Ok(this.dot(other))
        });
        methods.add_method("cross", |_lua, Self(this), LuaConstructVector(other)| {
            Ok(LuaVector(vector![
                this[1] * other[2] - this[2] * other[1],
                this[2] * other[0] - this[0] * other[2],
                this[0] * other[1] - this[1] * other[0],
            ]))
        });

        methods.add_method(
            "projected_to",
            |_lua, Self(this), LuaConstructVector(other)| match this.projected_to(&other) {
                Some(result) => Ok(LuaVector(result)),
                None => Err(LuaError::external("cannot project to zero vector")),
            },
        );
        methods.add_method(
            "rejected_from",
            |_lua, Self(this), LuaConstructVector(other)| match this.rejected_from(&other) {
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

        // Vector == Vector
        methods.add_meta_function(LuaMetaMethod::Eq, |_lua, (LuaVector(a), LuaVector(b))| {
            Ok(approx_eq(&a, &b))
        });

        // Vector[index]
        methods.add_meta_method(
            LuaMetaMethod::Index,
            |_lua, Self(this), LuaVectorIndex(index)| Ok(this.get(index)),
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

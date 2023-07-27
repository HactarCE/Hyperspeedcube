use itertools::Itertools;

use super::*;
use crate::math::*;

impl LuaUserData for Vector {
    fn add_methods<'lua, T: LuaUserDataMethods<'lua, Self>>(methods: &mut T) {
        methods.add_method("ndim", |_lua, this, ()| Ok(this.ndim()));
        methods.add_method("mag2", |_lua, this, ()| Ok(this.mag2()));
        methods.add_method("mag", |_lua, this, ()| Ok(this.mag()));

        methods.add_method("at_ndim", |_lua, this, LuaNdim(new_ndim)| {
            let mut ret = this.clone();
            ret.resize(new_ndim);
            Ok(ret)
        });

        methods.add_method_mut("set_ndim", |_lua, this, LuaNdim(new_ndim)| {
            this.resize(new_ndim);
            Ok(())
        });

        methods.add_method("normalized", |_lua, this, new_mag: Option<Float>| {
            let new_mag = new_mag.unwrap_or(1.0);
            match util::try_div(new_mag, this.mag()) {
                Some(scale) => Ok(this * scale),
                None => Ok(this.clone()),
            }
        });

        methods.add_method("projected_to", |lua, this, other_args| {
            let other = lua_construct_vector(lua, other_args)?;
            Ok(&other * (this.dot(&other) / other.mag2()))
        });

        // Vector + Vector
        methods.add_meta_function(
            LuaMetaMethod::Add,
            |_lua, (LuaVector(lhs), LuaVector(rhs))| Ok(lhs + rhs),
        );

        // Vector - Vector
        methods.add_meta_function(
            LuaMetaMethod::Sub,
            |_lua, (LuaVector(lhs), LuaVector(rhs))| Ok(lhs - rhs),
        );

        // Vector * f64; f64 * Vector
        methods.add_meta_function(
            LuaMetaMethod::Mul,
            |_lua, pair: (LuaVectorOrNumber, LuaVectorOrNumber)| {
                use LuaVectorOrNumber::{Num, Vec};
                match pair {
                    (Vec(v), Num(a)) | (Num(a), Vec(v)) => Ok(v * a),
                    (a, b) => Err(LuaError::external(format!(
                        "cannot multiply {} and {}",
                        a.type_name(),
                        b.type_name(),
                    ))),
                }
            },
        );

        // Vector / f64
        methods.add_meta_function(
            LuaMetaMethod::Div,
            |_lua, (LuaVector(lhs), rhs): (_, Float)| Ok(lhs / rhs),
        );

        // -Vector
        methods.add_meta_function(LuaMetaMethod::Unm, |_lua, LuaVector(v)| Ok(-v));

        // Vector == Vector
        methods.add_meta_function(LuaMetaMethod::Eq, |_lua, (LuaVector(a), LuaVector(b))| {
            Ok(approx_eq(&a, &b))
        });

        // Vector[index]
        methods.add_meta_method(LuaMetaMethod::Index, |_lua, this, LuaVectorIndex(index)| {
            Ok(this.get(index))
        });

        // Vector[index] = new_value
        methods.add_meta_method_mut(
            LuaMetaMethod::NewIndex,
            |_lua, this, (LuaVectorIndex(index), new_value): (_, Float)| {
                *this = this.pad(index + 1);
                this[index] = new_value;
                Ok(())
            },
        );

        // #Vector
        methods.add_meta_method(LuaMetaMethod::Len, |_lua, this, ()| Ok(this.ndim()));

        // tostring(Vector)
        methods.add_meta_method(LuaMetaMethod::ToString, |_lua, this, ()| {
            Ok(this.to_string())
        });

        // pairs(Vector)
        methods.add_meta_method(LuaMetaMethod::Pairs, |_lua, this, ()| {
            let vector_iter = lua_fn!(|_lua, (LuaVector(v), LuaVectorIndex(i))| {
                if i < v.ndim() {
                    // Add 2 because `LuaVectorIndex` subtracts 1 to be zero-indexed
                    Ok((Some(i + 2), Some(v[i])))
                } else {
                    Ok((None, None))
                }
            });

            Ok((vector_iter, this.clone(), 0))
        });
    }
}

pub fn lua_construct_vector<'lua>(
    lua: LuaContext<'lua>,
    (first, rest): (Option<LuaValue<'lua>>, LuaMultiValue<'lua>),
) -> LuaResult<Vector> {
    match first {
        None | Some(LuaValue::Integer(_)) | Some(LuaValue::Number(_)) => {
            itertools::chain(first, rest)
                .into_iter()
                .map(|v| Float::from_lua(v, lua))
                .try_collect()
        }

        Some(first) => lua_convert!(match (lua, &first, "number, vector, table, or axis name") {
            <Vector>(v) => Ok(v),
            <LuaTable>(t)  => {
                let mut ret = vector![];
                for pair in t.pairs() {
                    let (LuaVectorIndex(k), v): (_, Float) = pair?;
                    ret = ret.pad(k + 1);
                    ret[k] = v;
                }
                Ok(ret)
            },
            <LuaAxisName>(axis) => Ok(Vector::unit(axis.0)),
        }),
    }
}

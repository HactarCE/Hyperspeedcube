use hypermath::prelude::*;

use super::*;

#[derive(Debug, Clone)]
pub struct LuaMultivector(pub Multivector);

impl<'lua> FromLua<'lua> for LuaMultivector {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        match value {
            LuaValue::Nil => Ok(Self(Multivector::zero())),
            LuaValue::Table(t) => Self::construct_from_table(t),
            LuaValue::Integer(x) => Ok(Self(Multivector::scalar(x as _))),
            LuaValue::Number(x) => Ok(Self(Multivector::scalar(x as _))),
            LuaValue::String(s) => {
                let axes: LuaMultivectorIndex =
                    s.to_str()?
                        .parse()
                        .map_err(|e| LuaError::FromLuaConversionError {
                            from: "string",
                            to: "axes string",
                            message: Some(e),
                        })?;
                Ok(Self(axes.to_multivector()))
            }
            v => {
                if let Ok(m) = cast_userdata::<Self>(lua, &v) {
                    Ok(m)
                } else if let Ok(LuaVector(v)) = cast_userdata(lua, &v) {
                    Ok(Self(v.into()))
                } else if let Ok(axis) = cast_userdata::<LuaAxis>(lua, &v) {
                    Ok(Self(axis.vector()?.into()))
                } else {
                    lua_convert_err(&v, "multivector, vector, table, number, or axes string")
                }
            }
        }
    }
}

impl LuaMultivector {
    fn construct_from_table(t: LuaTable<'_>) -> LuaResult<LuaMultivector> {
        let mut ret = Multivector::ZERO;
        for pair in t.pairs() {
            let (k, v): (LuaMultivectorIndex, Float) = pair?;
            ret += k.to_multivector() * v;
        }
        Ok(LuaMultivector(ret))
    }
}

impl LuaUserData for LuaMultivector {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("multivector"));
    }

    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, |_lua, Self(this), ()| {
            Ok(format!("multivector({this})"))
        });

        methods.add_method("mag2", |_lua, Self(this), ()| Ok(this.mag2()));
        methods.add_method("mag", |_lua, Self(this), ()| Ok(this.mag()));

        methods.add_method("normalized", |_lua, Self(this), new_mag: Option<Float>| {
            let new_mag = new_mag.unwrap_or(1.0);
            match hypermath::util::try_div(new_mag, this.mag()) {
                Some(scale) => Ok(LuaMultivector(this * scale)),
                None => Ok(LuaMultivector(this.clone())),
            }
        });

        methods.add_method("ndim", |_lua, Self(this), ()| Ok(this.ndim()));
        methods.add_method("grade", |_lua, Self(this), ()| {
            match Blade::try_from(this.clone()) {
                Ok(blade) => Ok(Some(blade.grade())),
                Err(MismatchedGrade) => Ok(None),
            }
        });

        methods.add_method("reverse", |_lua, Self(this), ()| {
            Ok(LuaMultivector(this.reverse()))
        });
        methods.add_method("inverse", |_lua, Self(this), ()| {
            Ok(this.inverse().map(LuaMultivector))
        });

        methods.add_method("to_grade", |_lua, Self(this), LuaNdim(grade)| {
            Ok(LuaMultivector(this.clone().grade_project(grade).into_mv()))
        });

        // Multivector + Multivector
        methods.add_meta_function(
            LuaMetaMethod::Add,
            |_lua, (LuaMultivector(lhs), LuaMultivector(rhs))| Ok(LuaMultivector(lhs + rhs)),
        );

        // Multivector - Multivector
        methods.add_meta_function(
            LuaMetaMethod::Sub,
            |_lua, (LuaMultivector(lhs), LuaMultivector(rhs))| Ok(LuaMultivector(lhs - rhs)),
        );

        // Multivector * Multivector
        methods.add_meta_function(
            LuaMetaMethod::Mul,
            |_lua, (LuaMultivector(lhs), LuaMultivector(rhs))| Ok(LuaMultivector(lhs * rhs)),
        );

        // Multivector / Multivector
        methods.add_meta_function(
            LuaMetaMethod::Div,
            |_lua, (LuaMultivector(lhs), LuaMultivector(rhs))| {
                let rhs_inv = rhs
                    .inverse()
                    .ok_or(LuaError::external("multivector has no inverse"))?;
                Ok(LuaMultivector(lhs * rhs_inv))
            },
        );

        // -Multivector
        methods.add_meta_function(LuaMetaMethod::Unm, |_lua, LuaMultivector(m)| {
            Ok(LuaMultivector(-m))
        });

        // Multivector ^ Multivector
        methods.add_meta_function(
            LuaMetaMethod::Pow,
            |_lua, (LuaMultivector(lhs), LuaMultivector(rhs))| Ok(LuaMultivector(lhs ^ rhs)),
        );

        // ~Multivector
        methods.add_meta_function(LuaMetaMethod::BNot, |_lua, LuaMultivector(m)| {
            Ok(LuaMultivector(m.reverse()))
        });

        // Multivector << Multivector
        methods.add_meta_function(
            LuaMetaMethod::Shl,
            |_lua, (LuaMultivector(lhs), LuaMultivector(rhs))| Ok(LuaMultivector(lhs << rhs)),
        );

        // Multivector == Multivector
        methods.add_meta_function(
            LuaMetaMethod::Eq,
            |_lua, (LuaMultivector(a), LuaMultivector(b))| Ok(approx_eq(&a, &b)),
        );

        // Multivector[index]
        methods.add_meta_method(
            LuaMetaMethod::Index,
            |_lua, Self(this), axes: LuaMultivectorIndex| {
                Ok(match axes.nino {
                    Some(NiNo::No) => this.get_no(axes.axes),
                    Some(NiNo::Ni) => this.get_ni(axes.axes),
                    None => this.get(axes.axes).unwrap_or(0.0),
                } * axes.sign)
            },
        );

        // We do not add `LuaMetaMethod::NewIndex` because this can be used to
        // mutate aliased multivectors, which is very confusing.

        // #Multivector
        methods.add_meta_method(LuaMetaMethod::Len, |_lua, _this, ()| {
            Err::<(), _>(LuaError::external(
                "cannot take the length of a multivector; \
                 use `ndim` or `grade` instead",
            ))
        });

        // tostring(Multivector)
        methods.add_meta_method(LuaMetaMethod::ToString, |_lua, Self(this), ()| {
            Ok(this.to_string())
        });

        // pairs(Multivector)
        methods.add_meta_method(LuaMetaMethod::Pairs, |_lua, Self(this), ()| {
            struct TermsIterFn(std::vec::IntoIter<Term>);
            impl LuaUserData for TermsIterFn {
                fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
                    methods.add_meta_method_mut(
                        LuaMetaMethod::Call,
                        |_lua, this, _: LuaMultiValue<'_>| {
                            Ok(match this.0.next() {
                                Some(term) => (Some(term.axes.to_string()), Some(term.coef)),
                                None => (None, None),
                            })
                        },
                    );
                }
            }

            // The data needs to be owned so that we can return it from the
            // function.
            #[allow(clippy::unnecessary_to_owned)]
            Ok(TermsIterFn(this.terms().to_vec().into_iter()))
        });
    }
}
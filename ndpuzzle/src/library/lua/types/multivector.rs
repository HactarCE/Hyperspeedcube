use super::*;
use crate::math::{cga::*, *};

impl LuaUserData for Multivector {
    fn add_methods<'lua, T: LuaUserDataMethods<'lua, Self>>(methods: &mut T) {
        methods.add_method("mag2", |_lua, this, ()| Ok(this.mag2()));
        methods.add_method("mag", |_lua, this, ()| Ok(this.mag()));

        methods.add_method("normalized", |_lua, this, new_mag: Option<Float>| {
            let new_mag = new_mag.unwrap_or(1.0);
            match util::try_div(new_mag, this.mag()) {
                Some(scale) => Ok(this * scale),
                None => Ok(this.clone()),
            }
        });

        methods.add_method("ndim", |_lua, this, ()| Ok(this.ndim()));
        methods.add_method("grade", |_lua, this, ()| {
            match Blade::try_from(this.clone()) {
                Ok(blade) => Ok(Some(blade.grade())),
                Err(cga::MismatchedGrade) => Ok(None),
            }
        });

        methods.add_method("reverse", |_lua, this, ()| Ok(this.reverse()));
        methods.add_method("inverse", |_lua, this, ()| Ok(this.inverse()));

        // Multivector + Multivector
        methods.add_meta_function(
            LuaMetaMethod::Add,
            |_lua, (LuaMultivector(lhs), LuaMultivector(rhs))| Ok(lhs + rhs),
        );

        // Multivector - Multivector
        methods.add_meta_function(
            LuaMetaMethod::Sub,
            |_lua, (LuaMultivector(lhs), LuaMultivector(rhs))| Ok(lhs - rhs),
        );

        // Multivector * Multivector
        methods.add_meta_function(
            LuaMetaMethod::Mul,
            |_lua, (LuaMultivector(lhs), LuaMultivector(rhs))| Ok(lhs * rhs),
        );

        // Multivector / Multivector
        methods.add_meta_function(
            LuaMetaMethod::Div,
            |_lua, (LuaMultivector(lhs), LuaMultivector(rhs))| {
                let rhs_inv = rhs
                    .inverse()
                    .ok_or(LuaError::external("multivector has no inverse"))?;
                Ok(lhs * rhs_inv)
            },
        );

        // -Multivector
        methods.add_meta_function(LuaMetaMethod::Unm, |_lua, LuaMultivector(m)| Ok(-m));

        methods.add_meta_function(
            LuaMetaMethod::Pow,
            |lua, (LuaMultivector(lhs), LuaMultivector(rhs))| Ok(lhs ^ rhs),
        );
        methods.add_meta_function(
            LuaMetaMethod::BNot,
            |lua, LuaMultivector(m)| Ok(m.reverse()),
        );
        methods.add_meta_function(
            LuaMetaMethod::Shl,
            |lua, (LuaMultivector(lhs), LuaMultivector(rhs))| Ok(lhs << rhs),
        );

        // Multivector == Multivector
        methods.add_meta_function(
            LuaMetaMethod::Eq,
            |_lua, (LuaMultivector(a), LuaMultivector(b))| Ok(approx_eq(&a, &b)),
        );

        // Multivector[index]
        methods.add_meta_method(LuaMetaMethod::Index, |_lua, this, axes: LuaAxesString| {
            Ok(match axes.nino {
                Some(NiNo::No) => this.get_no(axes.axes),
                Some(NiNo::Ni) => this.get_ni(axes.axes),
                None => this.get(axes.axes).unwrap_or(0.0),
            } * axes.sign.to_float())
        });

        // Multivector[index] = new_value
        methods.add_meta_method_mut(
            LuaMetaMethod::NewIndex,
            |_lua, this, (axes, new_value): (LuaAxesString, Float)| {
                let LuaAxesString {
                    nino,
                    axes,
                    sign,
                    string,
                } = axes;
                if nino.is_some() {
                    return Err(LuaError::external(format!(
                        "cannot assign to component '{string}'",
                    )));
                }
                *this += Term {
                    coef: new_value * sign.to_float() - this[axes],
                    axes,
                };
                Ok(())
            },
        );

        // #Multivector
        methods.add_meta_method(LuaMetaMethod::Len, |_lua, _this, ()| {
            Err::<(), _>(LuaError::external(
                "cannot take the length of a multivector; \
                 use `ndim` or `grade` instead",
            ))
        });

        // tostring(Multivector)
        methods.add_meta_method(LuaMetaMethod::ToString, |_lua, this, ()| {
            Ok(this.to_string())
        });

        // pairs(Multivector)
        methods.add_meta_method(LuaMetaMethod::Pairs, |_lua, this, ()| {
            struct TermsIterFn(std::vec::IntoIter<Term>);
            impl LuaUserData for TermsIterFn {
                fn add_methods<'lua, T: LuaUserDataMethods<'lua, Self>>(methods: &mut T) {
                    methods.add_meta_method_mut(
                        LuaMetaMethod::Call,
                        |_lua, this, _: LuaMultiValue| {
                            Ok(match this.0.next() {
                                Some(term) => (Some(term.axes.to_string()), Some(term.coef)),
                                None => (None, None),
                            })
                        },
                    )
                }
            }

            Ok(TermsIterFn(this.terms().to_vec().into_iter()))
        });
    }
}

pub fn lua_construct_multivector<'lua>(
    lua: LuaContext<'lua>,
    (first, rest): (Option<LuaValue<'lua>>, LuaMultiValue<'lua>),
) -> LuaResult<Multivector> {
    match first {
        None | Some(LuaValue::Integer(_)) | Some(LuaValue::Number(_)) => {
            Ok(super::lua_construct_vector(lua, (first, rest))?.into())
        }
        Some(first) => lua_convert!(match (lua, &first, "number, vector, table, or axis name") {
            <Multivector>(m) => Ok(m),
            <Vector>(v) => Ok(v.into()),
            <LuaTable>(t)  => {
                let mut ret = Multivector::ZERO;
                for pair in t.pairs() {
                    let (k, v): (LuaAxesString, Float) = pair?;
                    ret += k.to_multivector() * v;
                }
                Ok(ret)
            },
            <LuaAxesString>(axes) => Ok(axes.to_multivector()),
        }),
    }
}

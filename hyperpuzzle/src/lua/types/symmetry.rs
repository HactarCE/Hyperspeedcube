use hypermath::{
    collections::{
        approx_hashmap::{ApproxHashMapKey, FloatHash, MultivectorHash, VectorHash},
        ApproxHashMap,
    },
    prelude::*,
};
use hypershape::prelude::*;
use itertools::Itertools;

use super::*;

#[derive(Debug, Clone)]
pub struct LuaSymmetry {
    pub schlafli: SchlafliSymbol,
    pub chiral: bool,
}

impl<'lua> FromLua<'lua> for LuaSymmetry {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        cast_userdata(lua, &value)
    }
}

impl From<SchlafliSymbol> for LuaSymmetry {
    fn from(schlafli: SchlafliSymbol) -> Self {
        Self {
            schlafli,
            chiral: false,
        }
    }
}

impl LuaSymmetry {
    pub fn construct_from_table(t: LuaTable<'_>) -> LuaResult<Self> {
        t.sequence_values()
            .try_collect()
            .map(SchlafliSymbol::from_indices)
            .map(LuaSymmetry::from)
    }

    fn orbit<T: Clone + ApproxHashMapKey>(
        &self,
        object: T,
        transform: fn(&Isometry, &T) -> T,
    ) -> Vec<(Isometry, T)> {
        let ret = self.schlafli.expand(object, transform);
        match self.chiral {
            true => ret
                .into_iter()
                .filter(|(t, _obj)| !t.is_reflection())
                .collect(),
            false => ret,
        }
    }

    fn orbit_lua_iter<'lua>(
        &self,
        lua: &'lua Lua,
        args: LuaMultiValue<'lua>,
    ) -> LuaResult<LuaFunction<'lua>> {
        let ndim = self.schlafli.ndim();

        let is_all_numbers = args.iter().all(|arg| arg.is_integer() || arg.is_number());

        let init: Vec<Transformable> = if is_all_numbers {
            vec![Transformable::Vector(LuaVector(
                self.vector_from_args(lua, args)?,
            ))]
        } else {
            args.iter()
                .map(|v| {
                    if v.is_string() {
                        let s = &v.to_string()?;
                        let vector = parse_wendy_krieger_vector(ndim, s)?;
                        Ok(Transformable::Vector(LuaVector(vector)))
                    } else {
                        Transformable::from_lua(v.clone(), lua)
                    }
                })
                .try_collect()?
        };

        let mut iter = self
            .orbit(init, |t, obj| {
                obj.iter()
                    .map(|v| v.transform(t).unwrap_or_else(|e| Transformable::Error(e)))
                    .collect()
            })
            .into_iter();
        lua.create_function_mut(move |lua, ()| {
            iter.find_map(move |(transform, objects)| {
                let mut values = vec![];
                match LuaTransform(transform).into_lua(lua) {
                    Ok(t) => values.push(t),
                    Err(e) => return Some(Err(e)),
                }
                for obj in objects {
                    match obj.into_lua(lua)? {
                        Ok(v) => values.push(v),
                        Err(e) => return Some(Err(e)),
                    }
                }
                Some(Ok(LuaMultiValue::from_vec(values)))
            })
            .unwrap_or_else(|| lua.pack_multi(LuaNil))
        })
    }

    fn vector_from_args<'lua>(
        &self,
        lua: &'lua Lua,
        args: LuaMultiValue<'lua>,
    ) -> LuaResult<Vector> {
        let s = &self.schlafli;
        if let Ok(string) = String::from_lua_multi(args.clone(), lua) {
            Ok(mirror_basis(s)? * parse_wendy_krieger_vector(s.ndim(), &string)?)
        } else if let Ok(LuaVector(v)) = <_>::from_lua_multi(args, lua) {
            Ok(v)
        } else {
            Err(LuaError::external(
                "expected vector constructor or coxeter vector string",
            ))
        }
    }
}

impl LuaUserData for LuaSymmetry {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("symmetry"));
    }

    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, |_lua, this, ()| {
            Ok(this
                .schlafli
                .indices()
                .iter()
                .map(|i| i.to_string())
                .join("o"))
        });

        methods.add_meta_method(LuaMetaMethod::Call, |lua, this, args: LuaMultiValue<'_>| {
            Ok(LuaMultiValue::from_vec(args.into_vec()))
        });

        methods.add_method("chiral", |_lua, Self { schlafli, .. }, ()| {
            Ok(Self {
                schlafli: schlafli.clone(),
                chiral: true,
            })
        });

        methods.add_method("orbit", |lua, this, args: LuaMultiValue<'_>| {
            this.orbit_lua_iter(lua, args)
        });

        methods.add_method("ndim", |_lua, this, ()| Ok(this.schlafli.ndim()));

        methods.add_method("vec", |lua, this, args| {
            Ok(LuaVector(this.vector_from_args(lua, args)?))
        });
    }
}

fn mirror_basis(s: &SchlafliSymbol) -> LuaResult<Matrix> {
    s.mirror_basis()
        .ok_or_else(|| LuaError::external("coxeter diagram matrix be invertible"))
}

fn parse_wendy_krieger_vector(ndim: u8, s: &str) -> LuaResult<Vector> {
    // TODO: normalization syntax?
    // if s.starts_with('|')&& s.ends_with('|') {
    //     s.strip_prefix('|').and_then(|s|s.strip_suffix('|'))
    // }
    if s.len() != ndim as usize {
        return Err(LuaError::external(format!(
            "expected coxeter vector of length {ndim}",
        )));
    }
    s.chars()
        .map(|c| match c {
            // Blame Wendy Krieger for this notation.
            // https://bendwavy.org/klitzing/explain/dynkin-notation.htm
            'o' => Ok(0.0),
            'x' => Ok(1.0),
            'q' => Ok(std::f64::consts::SQRT_2),
            'f' => Ok((5.0_f64.sqrt() + 1.0) * 0.5), // phi
            'u' => Ok(2.0),
            _ => Err(LuaError::external(
                "invalid character for coxeter vector. \
                 supported characters: [o, x, q, f, u]",
            )),
        })
        .collect()
}

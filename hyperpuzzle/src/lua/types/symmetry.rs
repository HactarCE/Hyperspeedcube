use hypermath::collections::ApproxHashMapKey;
use hypermath::prelude::*;
use hypershape::prelude::*;
use itertools::Itertools;

use super::*;

/// Lua symmetry object.
#[derive(Debug, Clone)]
pub struct LuaSymmetry {
    /// Base Coxeter group.
    pub coxeter: CoxeterGroup,
    /// Whether to discount symmetry elements that have a net reflection.
    pub chiral: bool,
}

impl<'lua> FromLua<'lua> for LuaSymmetry {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        cast_userdata(lua, &value)
    }
}

impl<T: Into<CoxeterGroup>> From<T> for LuaSymmetry {
    fn from(value: T) -> Self {
        Self {
            coxeter: value.into(),
            chiral: false,
        }
    }
}

impl LuaSymmetry {
    /// Constructs a symmetry object from a Lua value (string or table).
    pub fn construct_from_lua_value(v: LuaValue<'_>) -> LuaResult<Self> {
        fn split_alpha_number(s: &str) -> Option<(&str, u8)> {
            let (num_index, _) = s.match_indices(char::is_numeric).next()?;
            let num = s[num_index..].parse().ok()?;
            Some((&s[0..num_index], num))
        }

        match v {
            LuaValue::String(s) => match s.to_string_lossy().to_ascii_lowercase().as_str() {
                "e6" => FiniteCoxeterGroup::E6.try_into(),
                "e7" => FiniteCoxeterGroup::E7.try_into(),
                "e8" => FiniteCoxeterGroup::E8.try_into(),
                "f4" => FiniteCoxeterGroup::F4.try_into(),
                "g2" => FiniteCoxeterGroup::G2.try_into(),
                "h2" => FiniteCoxeterGroup::H2.try_into(),
                "h3" => FiniteCoxeterGroup::H3.try_into(),
                "h4" => FiniteCoxeterGroup::H4.try_into(),
                s => match split_alpha_number(&s) {
                    Some(("a", n)) => FiniteCoxeterGroup::A(n).try_into(),
                    Some(("b" | "c" | "bc", n)) => FiniteCoxeterGroup::B(n).try_into(),
                    Some(("d", n)) => FiniteCoxeterGroup::D(n).try_into(),
                    Some(("i", n)) => FiniteCoxeterGroup::I(n).try_into(),
                    _ => return Err(LuaError::external("unknown coxeter group string")),
                },
            },
            LuaValue::Table(t) => {
                let indices: Vec<usize> = t.sequence_values().try_collect()?;
                CoxeterGroup::new_linear(&indices)
            }
            _ => return lua_convert_err(&v, "string or table"),
        }
        .map(LuaSymmetry::from)
        .into_lua_err()
    }

    /// Returns the orbit of an object under the symmetry.
    pub fn orbit<T: ApproxHashMapKey + Clone + TransformByMotor>(
        &self,
        object: T,
    ) -> Vec<(pga::Motor, T)> {
        self.coxeter.orbit(object, self.chiral)
    }

    /// Returns the orbit of a collection of objects under the symmetry.
    pub fn orbit_lua_iter<'lua>(
        &self,
        lua: &'lua Lua,
        args: LuaMultiValue<'lua>,
    ) -> LuaResult<LuaFunction<'lua>> {
        let ndim = LuaNdim::get(lua)?;

        let is_all_numbers = args.iter().all(|arg| arg.is_integer() || arg.is_number());

        let init: Vec<Transformable> = if is_all_numbers {
            vec![Transformable::Blade(LuaBlade(pga::Blade::from_vector(
                ndim,
                self.vector_from_args(lua, args)?,
            )))]
        } else {
            args.iter()
                .map(|v| {
                    if v.is_string() {
                        let s = &v.to_string()?;
                        let vector = parse_wendy_krieger_vector(ndim, s)?;
                        Transformable::from_vector(lua, vector)
                    } else {
                        Transformable::from_lua(v.clone(), lua)
                    }
                })
                .try_collect()?
        };

        let mut iter = self.orbit(init).into_iter();
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
        let s = &self.coxeter;
        if let Ok(string) = String::from_lua_multi(args.clone(), lua) {
            Ok(s.mirror_basis().into_lua_err()?
                * parse_wendy_krieger_vector(s.mirror_count(), &string)?)
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
            Ok(this.coxeter.to_string())
        });

        methods.add_method("chiral", |_lua, Self { coxeter, .. }, ()| {
            Ok(Self {
                coxeter: coxeter.clone(),
                chiral: true,
            })
        });

        methods.add_method("orbit", |lua, this, args: LuaMultiValue<'_>| {
            this.orbit_lua_iter(lua, args)
        });

        methods.add_method("ndim", |_lua, this, ()| Ok(this.coxeter.min_ndim()));

        methods.add_method("vec", |lua, this, args| {
            Ok(LuaVector(this.vector_from_args(lua, args)?))
        });
    }
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

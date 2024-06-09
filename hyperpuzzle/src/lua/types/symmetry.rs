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

    /// Constructs a vector in the symmetry basis from Dynkin notation.
    pub fn dynkin_vector(&self, string: &str) -> LuaResult<Vector> {
        Ok(self.coxeter.mirror_basis().into_lua_err()?
            * parse_dynkin_notation(self.coxeter.mirror_count(), &string)?)
    }

    fn vector_from_args<'lua>(
        &self,
        lua: &'lua Lua,
        args: LuaMultiValue<'lua>,
    ) -> LuaResult<Vector> {
        if let Ok(string) = String::from_lua_multi(args.clone(), lua) {
            self.dynkin_vector(&string)
        } else if let Ok(LuaVector(v)) = <_>::from_lua_multi(args, lua) {
            Ok(self.coxeter.mirror_basis().into_lua_err()? * v)
        } else {
            Err(LuaError::external(
                "expected vector constructor or dynkin notation string",
            ))
        }
    }

    /// Returns a motor representing a sequence of mirror reflections, specified
    /// using indices into the symmetry's mirror.
    pub fn motor_for_mirror_seq(
        &self,
        mirror_sequence: impl IntoIterator<Item = usize>,
    ) -> LuaResult<pga::Motor> {
        let ndim = self.coxeter.min_ndim();
        let mirrors = self.coxeter.mirrors();
        mirror_sequence
            .into_iter()
            .map(|i| -> Result<pga::Motor, &str> {
                let mirror = mirrors.get(i).ok_or("mirror index out of range")?;
                Ok(mirror.motor(ndim))
            })
            .reduce(|a, b| Ok(a? * b?))
            .unwrap_or(Ok(pga::Motor::ident(ndim)))
            .into_lua_err()
    }
}

impl LuaUserData for LuaSymmetry {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("symmetry"));

        fields.add_field_method_get("ndim", |_lua, this| Ok(this.coxeter.min_ndim()));

        fields.add_field_method_get("mirror_vectors", |lua, this| {
            lua.create_sequence_from(
                this.coxeter
                    .mirrors()
                    .iter()
                    .map(|m| LuaVector(m.normal().clone())),
            )
        });

        fields.add_field_method_get("chiral", |_lua, Self { coxeter, .. }| {
            Ok(Self {
                coxeter: coxeter.clone(),
                chiral: true,
            })
        });
        fields.add_field_method_get("is_chiral", |_lua, Self { chiral, .. }| Ok(*chiral));
    }

    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, |_lua, this, ()| {
            Ok(this.coxeter.to_string())
        });

        methods.add_meta_method(LuaMetaMethod::Index, |_lua, this, index: String| {
            if !index.is_empty() && index.chars().all(|c| c == 'o' || c == 'x') {
                this.dynkin_vector(&index).map(LuaVector).map(Some)
            } else {
                Ok(None)
            }
        });

        methods.add_method("orbit", |lua, this, args: LuaMultiValue<'_>| {
            let objects: Vec<Transformable> = args
                .into_iter()
                .map(|arg| Transformable::from_lua(arg, lua))
                .try_collect()?;
            Ok(LuaOrbit::new(this.clone(), objects))
        });

        methods.add_method("vec", |lua, this, args| {
            Ok(LuaVector(this.vector_from_args(lua, args)?))
        });

        methods.add_method("thru", |lua, this, indices: LuaMultiValue<'_>| {
            let indices: Vec<usize> = indices
                .into_iter()
                .map(|v| LuaIndex::from_lua(v, lua).map(|LuaIndex(i)| i))
                .try_collect()?;
            this.motor_for_mirror_seq(indices).map(LuaTransform)
        });
    }
}

fn parse_dynkin_notation(ndim: u8, s: &str) -> LuaResult<Vector> {
    if s.len() != ndim as usize {
        return Err(LuaError::external(format!(
            "expected dynkin notation string of length {ndim}; {s:?} has length {}",
            s.len(),
        )));
    }
    s.chars()
        .map(|c| match c {
            // Source: https://web.archive.org/web/20230410033043/https://bendwavy.org/klitzing//explain/dynkin-notation.htm
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

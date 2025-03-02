use std::borrow::Cow;

use hypermath::collections::ApproxHashMapKey;
use hypermath::prelude::*;
use hypershape::prelude::*;
use itertools::Itertools;
use pga::Motor;
use smallvec::smallvec;

use super::*;

/// Error message when trying to do a Coxeter group operation on a non-Coxeter
/// group.
pub const BAD_GROUP_OPERATION_MSG: &str = "this group does not support this operation";

/// Lua symmetry object.
#[derive(Debug, Clone)]
pub enum LuaSymmetry {
    /// Coxeter group (optionally chiral).
    Coxeter {
        /// Base Coxeter group.
        coxeter: CoxeterGroup,
        /// Whether to discount symmetry elements that have a net reflection.
        chiral: bool,
    },
    /// Symmetry from generators.
    Custom {
        /// Generators of the symmetry group.
        generators: Vec<Motor>,
    },
}

impl FromLua for LuaSymmetry {
    fn from_lua(value: LuaValue, lua: &Lua) -> LuaResult<Self> {
        cast_userdata(lua, &value)
    }
}

impl<T: Into<CoxeterGroup>> From<T> for LuaSymmetry {
    fn from(value: T) -> Self {
        Self::Coxeter {
            coxeter: value.into(),
            chiral: false,
        }
    }
}

impl LuaSymmetry {
    /// Constructs a symmetry object from a Lua value (string or table)
    /// representing a Coxeter group.
    pub fn construct_from_cd(
        lua: &Lua,
        (v, basis): (LuaValue, Option<LuaValue>),
    ) -> LuaResult<Self> {
        fn split_alpha_number(s: &str) -> Option<(&str, u8)> {
            let (num_index, _) = s.match_indices(char::is_numeric).next()?;
            let num = s[num_index..].parse().ok()?;
            Some((&s[0..num_index], num))
        }

        let basis = match basis {
            Some(LuaValue::String(basis_vector_string)) => Some(
                basis_vector_string
                    .to_string_lossy()
                    .chars()
                    .map(|c| {
                        let LuaVectorIndex(axis) = c.to_string().parse().into_lua_err()?;
                        LuaResult::Ok(Vector::unit(axis))
                    })
                    .try_collect()?,
            ),
            Some(val @ LuaValue::Table(_)) => {
                let LuaSequence(basis_vector_values) = lua.unpack(val)?;
                Some(
                    basis_vector_values
                        .into_iter()
                        .map(|LuaVector(v)| v)
                        .collect(),
                )
            }
            Some(_) => {
                return Err(LuaError::external(
                    "basis must be string of vector chars or table of vectors",
                ));
            }
            None => None,
        };

        match v {
            LuaValue::String(s) => match s.to_string_lossy().to_ascii_lowercase().as_str() {
                "e6" => FiniteCoxeterGroup::E6,
                "e7" => FiniteCoxeterGroup::E7,
                "e8" => FiniteCoxeterGroup::E8,
                "f4" => FiniteCoxeterGroup::F4,
                "g2" => FiniteCoxeterGroup::G2,
                "h2" => FiniteCoxeterGroup::H2,
                "h3" => FiniteCoxeterGroup::H3,
                "h4" => FiniteCoxeterGroup::H4,
                s => match split_alpha_number(s) {
                    Some(("a", n)) => FiniteCoxeterGroup::A(n),
                    Some(("b" | "c" | "bc", n)) => FiniteCoxeterGroup::B(n),
                    Some(("d", n)) => FiniteCoxeterGroup::D(n),
                    Some(("i", n)) => FiniteCoxeterGroup::I(n),
                    _ => return Err(LuaError::external("unknown coxeter group string")),
                },
            }
            .coxeter_group(basis),

            LuaValue::Table(t) => {
                let indices: Vec<usize> = t.sequence_values().try_collect()?;
                CoxeterGroup::new_linear(&indices, basis)
            }

            _ => return lua_convert_err(&v, "string or table"),
        }
        .map(LuaSymmetry::from)
        .into_lua_err()
    }
    /// Returns the underlying Coxeter group, or returns an error if the group
    /// was not constructed as a Coxeter group.
    pub fn as_coxeter(&self) -> LuaResult<&CoxeterGroup> {
        match self {
            LuaSymmetry::Coxeter { coxeter, .. } => Ok(coxeter),
            LuaSymmetry::Custom { .. } => Err(LuaError::external(BAD_GROUP_OPERATION_MSG)),
        }
    }

    /// Constructs a symmetry object from a sequence of generators.
    pub fn construct_from_generators(lua: &Lua, generators: LuaTable) -> LuaResult<Self> {
        Ok(Self::Custom {
            generators: generators
                .sequence_values()
                .map(|value| {
                    let value: LuaValue = value?;
                    if let Ok(LuaTransform(t)) = lua.unpack(value.clone()) {
                        Ok(t)
                    } else if let Ok(t) = lua.unpack::<LuaTwist>(value.clone()) {
                        Ok(t.get()?.transform.clone())
                    } else {
                        lua_convert_err(&value, "transform or twist")
                    }
                })
                .try_collect()?,
        })
    }

    /// Returns the orbit of an object under the symmetry.
    pub fn orbit<T: ApproxHashMapKey + Clone + TransformByMotor>(
        &self,
        object: T,
    ) -> Vec<(GeneratorSequence, pga::Motor, T)> {
        match self {
            LuaSymmetry::Coxeter { coxeter, chiral } => coxeter.orbit(object, *chiral),
            LuaSymmetry::Custom { generators } => hypershape::orbit(
                &generators
                    .iter()
                    .enumerate()
                    .map(|(i, m)| (smallvec![i as u8], m.clone()))
                    .collect_vec(),
                object,
            ),
        }
    }

    /// Constructs a vector in the symmetry basis from Dynkin notation.
    pub fn dynkin_vector(&self, string: &str) -> LuaResult<Vector> {
        let coxeter = self.as_coxeter()?;

        Ok(coxeter.mirror_basis() * parse_dynkin_notation(coxeter.mirror_count(), string)?)
    }

    fn vector_from_args(&self, lua: &Lua, args: LuaMultiValue) -> LuaResult<Vector> {
        let coxeter = self.as_coxeter()?;

        if let Ok(string) = String::from_lua_multi(args.clone(), lua) {
            self.dynkin_vector(&string)
        } else if let Ok(LuaVector(v)) = <_>::from_lua_multi(args, lua) {
            Ok(coxeter.mirror_basis() * v)
        } else {
            Err(LuaError::external(
                "expected vector constructor or dynkin notation string",
            ))
        }
    }

    /// Returns the list of generators of the underlying group.
    ///
    /// In chiral groups, these generators may include reflections and so
    /// special care must be taken when using them.
    pub fn underlying_generators(&self) -> Cow<'_, [Motor]> {
        match self {
            LuaSymmetry::Coxeter { coxeter, .. } => Cow::Owned(coxeter.generators()),
            LuaSymmetry::Custom { generators } => Cow::Borrowed(generators),
        }
    }
    /// Returns the list of generators of the group. In chiral groups, this
    /// returns a set of generators that does not include reflections.
    pub fn chiral_safe_generators(&self) -> Cow<'_, [Motor]> {
        match self {
            LuaSymmetry::Coxeter {
                coxeter,
                chiral: true,
            } => {
                let gens = coxeter.generators();
                let n = gens.len();
                Cow::Owned((1..n).map(|i| &gens[0] * &gens[i]).collect_vec())
            }
            _ => self.underlying_generators(),
        }
    }
    /// Returns the minimum number of dimensions required to represent the
    /// symmetry group.
    pub fn ndim(&self) -> u8 {
        match self {
            LuaSymmetry::Coxeter { coxeter, .. } => coxeter.min_ndim(),
            LuaSymmetry::Custom { generators } => {
                generators.iter().map(|g| g.ndim()).max().unwrap_or(1)
            }
        }
    }

    /// Returns a motor representing a sequence of generators, specified using
    /// indices into the list of generators.
    pub fn motor_for_gen_seq(
        &self,
        gen_seq: impl IntoIterator<Item = u8>,
    ) -> LuaResult<pga::Motor> {
        let generators = self.underlying_generators();
        let ndim = self.ndim();
        gen_seq
            .into_iter()
            .map(|i| -> Result<pga::Motor, &str> {
                let g = generators
                    .get(i as usize)
                    .ok_or("generator index out of range")?;
                Ok(g.to_ndim_at_least(ndim))
            })
            .reduce(|a, b| Ok(a? * b?))
            .unwrap_or(Ok(pga::Motor::ident(ndim)))
            .into_lua_err()
    }
}

impl LuaUserData for LuaSymmetry {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_meta_field("type", LuaStaticStr("symmetry"));

        fields.add_field_method_get("ndim", |_lua, this| match this {
            LuaSymmetry::Coxeter { coxeter, .. } => Ok(coxeter.min_ndim()),
            LuaSymmetry::Custom { generators } => {
                Ok(generators.iter().map(|g| g.ndim()).max().unwrap_or(1))
            }
        });

        fields.add_field_method_get("mirror_vectors", |lua, this| {
            let LuaSymmetry::Coxeter { coxeter, .. } = this else {
                return Err(LuaError::external(BAD_GROUP_OPERATION_MSG));
            };

            lua.create_sequence_from(
                coxeter
                    .mirrors()
                    .iter()
                    .map(|m| LuaVector(m.normal().clone())),
            )
        });

        fields.add_field_method_get("chiral", |_lua, this| {
            Ok(Self::Coxeter {
                coxeter: this.as_coxeter()?.clone(),
                chiral: true,
            })
        });
        fields.add_field_method_get("is_chiral", |_lua, this| match this {
            LuaSymmetry::Coxeter { chiral, .. } => Ok(Some(*chiral)),
            LuaSymmetry::Custom { .. } => Ok(None),
        });
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, |_lua, this, ()| match this {
            LuaSymmetry::Coxeter { coxeter, chiral } => {
                let mut s = coxeter.to_string();
                if *chiral {
                    s += " (chiral)";
                }
                Ok(s)
            }
            LuaSymmetry::Custom { generators } => {
                Ok(format!("symmetry({})", generators.iter().join(", ")))
            }
        });

        methods.add_meta_method(LuaMetaMethod::Index, |_lua, this, index: String| {
            if !index.is_empty() && index.chars().all(|c| c == 'o' || c == 'x') {
                this.dynkin_vector(&index).map(LuaVector).map(Some)
            } else {
                Ok(None)
            }
        });

        methods.add_method("orbit", |lua, this, args: LuaMultiValue| {
            let objects: Vec<Transformable> = args
                .into_iter()
                .map(|arg| Transformable::from_lua(arg, lua))
                .try_collect()?;
            Ok(LuaOrbit::new(this.clone(), objects))
        });

        methods.add_method("vec", |lua, this, args| {
            Ok(LuaVector(this.vector_from_args(lua, args)?))
        });

        methods.add_method("thru", |lua, this, indices: LuaMultiValue| {
            let indices: Vec<u8> = indices
                .into_iter()
                .map(|v| LuaIndex::from_lua(v, lua).map(|LuaIndex(i)| i as u8))
                .try_collect()?;
            this.motor_for_gen_seq(indices).map(LuaTransform)
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

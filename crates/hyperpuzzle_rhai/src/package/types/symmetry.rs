//! Rhai symmetry type.

use std::borrow::Cow;

use hypermath::pga::Motor;
use hypermath::{ApproxHashMapKey, IndexNewtype, TransformByMotor, Vector, VectorRef};
use hypershape::{
    AbbrGenSeq, CoxeterGroup, FiniteCoxeterGroup, GenSeq, GeneratorId, IsometryGroup,
};
use itertools::Itertools;
use rhai::Array;

use super::name_strategy::RhaiNameStrategy;
use super::*;

pub fn init_engine(engine: &mut Engine) {
    engine.register_type_with_name::<RhaiSymmetry>("symmetry");
}

pub fn register(module: &mut Module) {
    // Display
    new_fn("to_string").set_into_module(module, |s: &mut RhaiSymmetry| match s {
        RhaiSymmetry::Coxeter { coxeter, chiral } => format!(
            "<{}D {}coxeter group {:?}>",
            coxeter.min_ndim(),
            if *chiral { "chiral " } else { "" },
            coxeter.coxeter_matrix(),
        ),
        RhaiSymmetry::Custom { generators } => {
            format!(
                "<{}D group from {} generators>",
                generators.iter().map(|g| g.ndim()).max().unwrap_or(0),
                generators.len(),
            )
        }
    });
    new_fn("to_debug").set_into_module(module, |s: &mut RhaiSymmetry| format!("{s:?}"));

    // Constructors
    new_fn("cd").set_into_module(module, |name: String| -> Result<RhaiSymmetry> {
        RhaiSymmetry::construct_from_cd_str(&name, None)
    });
    new_fn("cd").set_into_module(
        module,
        |name: String, basis: String| -> Result<RhaiSymmetry> {
            RhaiSymmetry::construct_from_cd_str(&name, Some(basis_from_str(&basis)?))
        },
    );
    new_fn("cd").set_into_module(
        module,
        |ctx: Ctx<'_>, name: String, basis: Array| -> Result<RhaiSymmetry> {
            RhaiSymmetry::construct_from_cd_str(&name, Some(from_rhai_array(&ctx, basis)?))
        },
    );
    new_fn("cd").set_into_module(
        module,
        |ctx: Ctx<'_>, indices: Array| -> Result<RhaiSymmetry> {
            let indices = vec_from_rhai_array(&ctx, indices)?;
            RhaiSymmetry::construct_from_schalfli(&indices, None)
        },
    );
    new_fn("cd").set_into_module(
        module,
        |ctx: Ctx<'_>, indices: Array, basis: String| -> Result<RhaiSymmetry> {
            let indices = vec_from_rhai_array(&ctx, indices)?;
            RhaiSymmetry::construct_from_schalfli(&indices, Some(basis_from_str(&basis)?))
        },
    );
    new_fn("cd").set_into_module(
        module,
        |ctx: Ctx<'_>, indices: Array, basis: Array| -> Result<RhaiSymmetry> {
            let indices = vec_from_rhai_array(&ctx, indices)?;
            RhaiSymmetry::construct_from_schalfli(&indices, Some(from_rhai_array(&ctx, basis)?))
        },
    );
    new_fn("symmetry").set_into_module(
        module,
        |ctx: Ctx<'_>, generators: Array| -> Result<RhaiSymmetry> {
            RhaiSymmetry::construct_from_generators(from_rhai_array(&ctx, generators)?)
        },
    );

    // Getters
    FuncRegistration::new_getter("ndim")
        .set_into_module(module, |s: &mut RhaiSymmetry| -> i64 { s.ndim().into() });
    FuncRegistration::new_getter("mirror_vectors").set_into_module(
        module,
        |s: &mut RhaiSymmetry| -> Result<Array> {
            Ok(Array::from_iter(
                s.as_coxeter()?
                    .mirrors()
                    .iter()
                    .map(|m| Dynamic::from(m.normal().clone())),
            ))
        },
    );
    FuncRegistration::new_getter("chiral").set_into_module(
        module,
        |s: &mut RhaiSymmetry| -> Result<RhaiSymmetry> {
            Ok(RhaiSymmetry::Coxeter {
                coxeter: s.as_coxeter()?.clone(),
                chiral: true,
            })
        },
    );
    FuncRegistration::new_getter("is_chiral").set_into_module(
        module,
        |s: &mut RhaiSymmetry| -> bool {
            match s {
                RhaiSymmetry::Coxeter { chiral, .. } => *chiral,
                RhaiSymmetry::Custom { generators } => {
                    generators.iter().all(|g| !g.is_reflection())
                }
            }
        },
    );

    // Indexing
    FuncRegistration::new_index_getter().set_into_module(
        module,
        |s: &mut RhaiSymmetry, dynkin_string: String| -> Result<Vector> {
            s.dynkin_vector(&dynkin_string)
                .map_err(|e| format!("unknown property or invalid dynkin string: {e}").into())
        },
    );

    // Vector construction
    new_fn("vec").set_into_module(
        module,
        |s: &mut RhaiSymmetry, dynkin_string: String| -> Result<Vector> {
            s.dynkin_vector(&dynkin_string)
        },
    );
    new_fn("vec").set_into_module(
        module,
        |s: &mut RhaiSymmetry, v: Vector| -> Result<Vector> { s.vector(v) },
    );
    new_fn("vec").set_into_module(
        module,
        |ctx: Ctx<'_>, s: &mut RhaiSymmetry, a: Array| -> Result<Vector> {
            s.vector(super::vector::try_collect_to_vector(&ctx, &a)?)
        },
    );

    new_fn("thru").set_into_module(
        module,
        |s: &mut RhaiSymmetry, index: i64| -> Result<Motor> {
            let generator_id = u8::try_from(index).map_err(|e| e.to_string())?;
            s.motor_for_gen_seq(&GenSeq::new([GeneratorId(generator_id)]))
        },
    );
    new_fn("thru").set_into_module(
        module,
        |ctx: Ctx<'_>, s: &mut RhaiSymmetry, a: Array| -> Result<Motor> {
            let generator_ids = from_rhai(&ctx, Dynamic::from(a))?;
            s.motor_for_gen_seq(&generator_ids)
        },
    );
}

/// Error message when trying to do a Coxeter group operation on a non-Coxeter
/// group.
pub const BAD_GROUP_OPERATION_MSG: &str = "this group does not support this operation";

/// Lua symmetry object.
#[derive(Debug, Clone)]
pub enum RhaiSymmetry {
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

crate::impl_from_rhai!(RhaiSymmetry, "symmetry");

fn basis_from_str(s: &str) -> Result<Vec<Vector>> {
    s.chars()
        .map(|c| match hypermath::axis_from_char(c) {
            Some(axis) => Ok(Vector::unit(axis)),
            None => Err(format!("invalid axis symbol {c:?}").into()),
        })
        .collect()
}

impl RhaiSymmetry {
    /// Returns the current symmetry, set using `with symmetry { ... }`.
    pub fn get(ctx: impl RhaiCtx) -> Option<Self> {
        RhaiState::get(ctx).lock().symmetry.clone()
    }

    /// Constructs a symmetry object from a Coxeter string such as `"bc3"`.
    pub fn construct_from_cd_str(s: &str, basis: Option<Vec<Vector>>) -> Result<Self> {
        fn split_alpha_number(s: &str) -> Option<(&str, u8)> {
            let (num_index, _) = s.match_indices(char::is_numeric).next()?;
            let num = s[num_index..].parse().ok()?;
            Some((&s[0..num_index], num))
        }

        let coxeter = match s.to_ascii_lowercase().as_str() {
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
                _ => return Err("unknown coxeter group string".into()),
            },
        }
        .coxeter_group(basis)
        .map_err(|e| e.to_string())?;

        Ok(RhaiSymmetry::Coxeter {
            coxeter,
            chiral: false,
        })
    }

    /// Constructs a symmetry object from a Schalfli symbol such as `[4, 3]`.
    pub fn construct_from_schalfli(indices: &[usize], basis: Option<Vec<Vector>>) -> Result<Self> {
        let coxeter = CoxeterGroup::new_linear(indices, basis).map_err(|e| e.to_string())?;
        Ok(RhaiSymmetry::Coxeter {
            coxeter,
            chiral: false,
        })
    }

    /// Returns the underlying Coxeter group, or returns an error if the group
    /// was not constructed as a Coxeter group.
    pub fn as_coxeter(&self) -> Result<&CoxeterGroup> {
        match self {
            RhaiSymmetry::Coxeter { coxeter, .. } => Ok(coxeter),
            RhaiSymmetry::Custom { .. } => Err(BAD_GROUP_OPERATION_MSG.into()),
        }
    }

    /// Constructs a symmetry object from a sequence of generators.
    pub fn construct_from_generators(generators: Vec<Motor>) -> Result<Self> {
        Ok(Self::Custom { generators })
    }

    /// Constructs a vector in the symmetry basis from Dynkin notation.
    pub fn dynkin_vector(&self, string: &str) -> Result<Vector> {
        let coxeter = self.as_coxeter()?;
        Ok(coxeter.mirror_basis()
            * hypershape::group::parse_dynkin_notation(coxeter.mirror_count(), string)
                .map_err(|e| e.to_string())?)
    }

    pub fn vector(&self, v: impl VectorRef) -> Result<Vector> {
        let coxeter = self.as_coxeter()?;
        Ok(coxeter.mirror_basis() * v)
    }

    /// Returns the list of generators of the underlying group.
    ///
    /// In chiral groups, these generators may include reflections and so
    /// special care must be taken when using them.
    pub fn underlying_generators(&self) -> Cow<'_, [Motor]> {
        match self {
            RhaiSymmetry::Coxeter { coxeter, .. } => Cow::Owned(coxeter.generators()),
            RhaiSymmetry::Custom { generators } => Cow::Borrowed(generators),
        }
    }
    /// Returns the list of generators of the group. In chiral groups, this
    /// returns a set of generators that does not include reflections.
    pub fn chiral_safe_generators(&self) -> Cow<'_, [Motor]> {
        match self {
            RhaiSymmetry::Coxeter {
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
            RhaiSymmetry::Coxeter { coxeter, .. } => coxeter.min_ndim(),
            RhaiSymmetry::Custom { generators } => {
                generators.iter().map(|g| g.ndim()).max().unwrap_or(1)
            }
        }
    }

    /// Returns a motor representing a sequence of generators, specified using
    /// indices into the list of generators.
    pub fn motor_for_gen_seq(&self, gen_seq: &GenSeq) -> Result<Motor> {
        let generators = self.underlying_generators();
        let ndim = self.ndim();
        gen_seq
            .0
            .iter()
            .map(|&GeneratorId(i)| -> Result<Motor> {
                let g = generators
                    .get(i as usize)
                    .ok_or_else(|| format!("generator index {i} out of range"))?;
                Ok(g.to_ndim_at_least(ndim))
            })
            .reduce(|a, b| Ok(a? * b?))
            .unwrap_or(Ok(Motor::ident(ndim))) // if `gen_seq` is empty
    }

    /// Returns the orbit of an object under the symmetry.
    pub fn orbit<T: ApproxHashMapKey + Clone + TransformByMotor>(
        &self,
        object: T,
    ) -> Vec<(AbbrGenSeq, Motor, T)> {
        match self {
            RhaiSymmetry::Coxeter { coxeter, chiral } => coxeter.orbit(object, *chiral),
            RhaiSymmetry::Custom { generators } => hypershape::orbit(
                &generators
                    .iter()
                    .enumerate()
                    .map(|(i, m)| (GenSeq::new([GeneratorId(i as u8)]), m.clone()))
                    .collect_vec(),
                object,
            ),
        }
    }

    /// Returns the orbit of an object under the symmetry, with names specified
    /// using some name strategy.
    pub fn orbit_with_names<I: IndexNewtype, T: ApproxHashMapKey + Clone + TransformByMotor>(
        &self,
        ctx: &Ctx<'_>,
        object: T,
        names: &RhaiNameStrategy,
    ) -> Result<Vec<(AbbrGenSeq, Motor, T, Option<String>)>> {
        let name_fn = names.name_fn::<I, T>(ctx, Some(self), &object)?;
        self.orbit(object)
            .into_iter()
            .map(|(gen_seq, motor, obj)| {
                let name = name_fn.call(ctx, &motor, &obj)?;
                Ok((gen_seq, motor, obj, name))
            })
            .collect()
    }

    /// Constructs the isometry group.
    pub fn isometry_group(&self) -> Result<IsometryGroup> {
        IsometryGroup::from_generators(&self.chiral_safe_generators())
            .map_err(|e| e.to_string().into())
    }
}

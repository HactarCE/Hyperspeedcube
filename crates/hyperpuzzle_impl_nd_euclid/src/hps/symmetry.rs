//! Rhai symmetry type.

use std::fmt;
use std::sync::Arc;

use hypermath::pga::Motor;
use hypermath::{ApproxHashMapKey, Point, TransformByMotor, Vector, VectorRef, approx_eq};
use hyperpuzzlescript::{ErrorExt, Result, Span, Str, ValueData, hps_fns, impl_simple_custom_type};
use hypershape::{AbbrGenSeq, CoxeterGroup, FiniteCoxeterGroup, GenSeq, GeneratorId};
use itertools::Itertools;

impl_simple_custom_type!(
    HpsSymmetry = "euclid.Symmetry",
    |(this, this_span), (field, field_span)| match field {
        "ndim" => Some(this.ndim().into()),
        "chiral" => Some(this.chiral_subgroup().into()),
        "is_chiral" => Some(this.is_chiral().into()),
        "mirror_vectors" => Some(
            this.as_coxeter(this_span)?
                .mirrors()
                .iter()
                .map(|m| ValueData::Vec(m.normal().clone()).at(field_span))
                .collect_vec()
                .into()
        ),
        "generators" => Some(
            this.generators
                .iter()
                .map(|g| ValueData::EuclidTransform(g.clone()).at(field_span))
                .collect_vec()
                .into()
        ),
        s if s.chars().all(|c| "ox".contains(c)) =>
            Some(this.coxeter_dynkin_vector(this_span, s, field_span)?.into()),
        _ => None,
    },
);
impl fmt::Debug for HpsSymmetry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}
impl fmt::Display for HpsSymmetry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.coxeter_group {
            Some(coxeter) => {
                write!(
                    f,
                    "<{}D coxeter group {:?}>",
                    coxeter.min_ndim(),
                    coxeter.coxeter_matrix(),
                )
            }
            None => {
                write!(
                    f,
                    "<{}D group from {} generators>",
                    self.generators.iter().map(|g| g.ndim()).max().unwrap_or(1),
                    self.generators.len(),
                )
            }
        }
    }
}

/// Adds the built-ins to the scope.
pub fn define_in(scope: &hyperpuzzlescript::Scope) -> hyperpuzzlescript::Result<()> {
    scope.register_custom_type::<HpsSymmetry>();

    scope.register_builtin_functions(hps_fns![
        fn cd((name, name_span): Str) -> HpsSymmetry {
            HpsSymmetry::from_cd_str(&name, name_span)?
        }
        fn cd((indices, indices_span): Vec<usize>) -> HpsSymmetry {
            HpsSymmetry::from_schlafli(&indices, indices_span)?
        }
        fn symmetry(generators: Vec<Motor>) -> HpsSymmetry {
            HpsSymmetry::from_generators(generators)
        }

        fn vec((sym, sym_span): HpsSymmetry, (string, string_span): Str) -> Vector {
            sym.coxeter_dynkin_vector(sym_span, &string, string_span)?
        }
        fn vec((sym, sym_span): HpsSymmetry, (vec, vec_span): Vector) -> Vector {
            sym.coxeter_vector(sym_span, vec, vec_span)?
        }

        fn point((sym, sym_span): HpsSymmetry, (string, string_span): Str) -> Point {
            Point(sym.coxeter_dynkin_vector(sym_span, &string, string_span)?)
        }
        fn point((sym, sym_span): HpsSymmetry, (vec, vec_span): Vector) -> Point {
            Point(sym.coxeter_vector(sym_span, vec, vec_span)?)
        }

        fn thru(sym: HpsSymmetry, (index, index_span): u8) -> Motor {
            let gen_seq = GenSeq::new([GeneratorId(index)]);
            sym.motor_for_gen_seq(&gen_seq, index_span)?
        }
        fn thru(sym: HpsSymmetry, (indices, indices_span): Vec<u8>) -> Motor {
            let gen_seq = GenSeq::new(indices.into_iter().map(GeneratorId));
            sym.motor_for_gen_seq(&gen_seq, indices_span)?
        }
    ])
}

/// Symmetry group in N-dimensional Euclidean space.
///
/// This type is cheap to clone.
#[derive(Clone)]
pub struct HpsSymmetry {
    /// Generators of the group.
    generators: Arc<Vec<Motor>>,
    /// Coxeter group, if this is one.
    coxeter_group: Option<Arc<CoxeterGroup>>,
}

impl From<CoxeterGroup> for HpsSymmetry {
    fn from(coxeter: CoxeterGroup) -> Self {
        Self::from_coxeter(coxeter)
    }
}

impl PartialEq for HpsSymmetry {
    fn eq(&self, other: &Self) -> bool {
        self.generators.len() == other.generators.len()
            && std::iter::zip(&*self.generators, &*other.generators)
                .all(|(g1, g2)| approx_eq(g1, g2))
    }
}

impl HpsSymmetry {
    /// Constructs a symmetry object from a Coxeter group.
    pub fn from_coxeter(coxeter: CoxeterGroup) -> Self {
        Self {
            generators: Arc::new(coxeter.generators()),
            coxeter_group: Some(Arc::new(coxeter)),
        }
    }

    /// Constructs a symmetry object from a list of generators.
    pub fn from_generators(generators: impl IntoIterator<Item = Motor>) -> Self {
        Self {
            generators: Arc::new(generators.into_iter().collect()),
            coxeter_group: None,
        }
    }

    /// Constructs a symmetry object from a Coxeter string such as `"bc3"`.
    pub fn from_cd_str(s: &str, span: Span) -> Result<Self> {
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
                _ => return Err("unknown coxeter group string".at(span)),
            },
        }
        .coxeter_group(None)
        .at(span)?;
        Ok(coxeter.into())
    }

    /// Constructs a symmetry object from a Schalfli symbol such as `[4, 3]`.
    pub fn from_schlafli(indices: &[usize], span: Span) -> Result<Self> {
        Ok(CoxeterGroup::new_linear(indices, None).at(span)?.into())
    }

    /// Returns the underlying Coxeter group, or returns an error if the group
    /// was not constructed as a Coxeter group.
    pub fn as_coxeter(&self, span: Span) -> Result<&CoxeterGroup> {
        self.coxeter_group
            .as_deref()
            .ok_or("expected Coxeter group")
            .at(span)
    }

    /// Returns whether any of the generators of the group is a reflection.
    pub fn is_chiral(&self) -> bool {
        self.generators.iter().all(|g| !g.is_reflection())
    }
    /// Returns the chiral subgroup of the group.
    ///
    /// This is idempotent.
    pub fn chiral_subgroup(&self) -> Self {
        let rotations = self.generators.iter().filter(|g| !g.is_reflection());
        let mut reflections = self.generators.iter().filter(|g| g.is_reflection());
        let first_reflection = reflections.next();

        Self::from_generators(itertools::chain(
            rotations.cloned(),
            reflections.filter_map(|r| Some(first_reflection? * r)),
        ))
    }

    /// Constructs a vector in the symmetry basis from Dynkin notation.
    pub fn coxeter_dynkin_vector(
        &self,
        self_span: Span,
        string: &str,
        string_span: Span,
    ) -> Result<Vector> {
        let v = hypershape::group::parse_dynkin_notation(self.generators.len() as u8, string)
            .at(string_span)?;
        self.coxeter_vector(self_span, v, string_span)
    }
    /// Constructs a vector in the symmetry basis from the
    pub fn coxeter_vector(
        &self,
        self_span: Span,
        v: impl VectorRef,
        v_span: Span,
    ) -> Result<Vector> {
        let mirror_basis = self.as_coxeter(self_span)?.mirror_basis();

        let v_ndim = v.ndim();
        let basis_ndim = mirror_basis.ndim();
        if v_ndim > basis_ndim {
            return Err(
                format!("group has ndim {basis_ndim} but vector {v:?} has ndim {v_ndim}")
                    .at(v_span),
            );
        }

        Ok(mirror_basis * v)
    }

    /// Returns the minimum number of dimensions required to represent the
    /// symmetry group.
    pub fn ndim(&self) -> u8 {
        self.generators.iter().map(|g| g.ndim()).max().unwrap_or(1)
    }

    /// Returns a motor representing a sequence of generators, specified using
    /// indices into the list of generators.
    pub fn motor_for_gen_seq(&self, gen_seq: &GenSeq, span: Span) -> Result<Motor> {
        let ndim = self.ndim();
        gen_seq
            .0
            .iter()
            .map(|&GeneratorId(i)| -> Result<Motor> {
                let g = self
                    .generators
                    .get(i as usize)
                    .ok_or_else(|| format!("generator index {i} out of range").at(span))?;
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
        hypershape::orbit(
            &self
                .generators
                .iter()
                .enumerate()
                .map(|(i, m)| (GenSeq::new([GeneratorId(i as u8)]), m.clone()))
                .collect_vec(),
            object,
        )
    }
}

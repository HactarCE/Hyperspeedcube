//! HPS symmetry type.

use std::fmt;
use std::sync::Arc;

use hypergroup::{
    AbbrGenSeq, CoxeterMatrix, GenSeq, GeneratorId, GroupError, GroupResult, IsometryGroup,
};
use hypermath::pga::Motor;
use hypermath::prelude::*;
use hyperpuzzlescript::{
    Builtins, ErrorExt, EvalCtx, Result, Span, Spanned, Str, ValueData, hps_fns,
    impl_simple_custom_type,
};
use itertools::Itertools;

/// Symmetry group in N-dimensional Euclidean space.
///
/// This type is cheap to clone.
#[derive(Clone)]
pub struct HpsSymmetry {
    /// Generators of the group.
    generators: Arc<Vec<Motor>>,
    /// Coxeter matrix, if this is one.
    coxeter_group: Option<Arc<CoxeterMatrix>>,
    /// Offset by which the whole Coxeter group is transformed.
    coxeter_offset: Motor,
}
impl_simple_custom_type!(
    HpsSymmetry = "euclid.Symmetry",
    field_get = Self::impl_field_get,
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
                write!(f, "<{coxeter}>")
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

/// Adds the built-ins.
pub fn define_in(builtins: &mut Builtins<'_>) -> hyperpuzzlescript::Result<()> {
    builtins.set_custom_ty::<HpsSymmetry>()?;

    builtins.namespace("euclid")?.set_fns(hps_fns![
        fn cd((name, name_span): Str) -> HpsSymmetry {
            HpsSymmetry::from_cd_str(&name, name_span)?
        }
        fn cd((indices, indices_span): Vec<u16>) -> HpsSymmetry {
            HpsSymmetry::from_schlafli(&indices, indices_span)?
        }
        fn symmetry(generators: Vec<Motor>) -> HpsSymmetry {
            HpsSymmetry::from_generators(generators)
        }
    ])?;

    builtins.set_fns(hps_fns![
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

pub(super) fn orbit_spanned<T>(
    ctx: &mut EvalCtx<'_>,
    sym: HpsSymmetry,
    object: T,
) -> Vec<Spanned<T>>
where
    T: ApproxHash + Clone + Ndim + TransformByMotor,
{
    sym.orbit(object)
        .into_iter()
        .map(|(_, _, obj)| (obj, ctx.caller_span))
        .collect()
}

impl TryFrom<CoxeterMatrix> for HpsSymmetry {
    type Error = GroupError;

    fn try_from(value: CoxeterMatrix) -> std::result::Result<Self, Self::Error> {
        Self::from_coxeter(value)
    }
}

impl PartialEq for HpsSymmetry {
    fn eq(&self, other: &Self) -> bool {
        self.generators.len() == other.generators.len()
            && std::iter::zip(&*self.generators, &*other.generators)
                .all(|(g1, g2)| APPROX.eq(g1, g2))
    }
}

impl Ndim for HpsSymmetry {
    /// Returns the minimum number of dimensions required to represent the
    /// symmetry group.
    fn ndim(&self) -> u8 {
        self.generators.iter().map(|g| g.ndim()).max().unwrap_or(1)
    }
}

impl TransformByMotor for HpsSymmetry {
    fn transform_by(&self, m: &Motor) -> Self {
        let mut ret = self.clone();
        for g in Arc::make_mut(&mut ret.generators) {
            *g = m.transform(g);
        }
        ret.coxeter_offset = m * &ret.coxeter_offset;
        ret
    }
}

impl HpsSymmetry {
    pub fn get<'a>(ctx: &EvalCtx<'a>) -> Result<Option<&'a Self>> {
        ctx.scope.special.sym.ref_to()
    }

    fn impl_field_get(
        &self,
        _span: Span,
        (field, field_span): Spanned<&str>,
    ) -> Result<Option<ValueData>> {
        Ok(match field {
            "ndim" => Some(self.ndim().into()),
            "chiral" => Some(self.chiral_subgroup().into()),
            "is_chiral" => Some(self.is_chiral().into()),
            "mirror_vectors" => Some(
                self.as_coxeter(field_span)?
                    .mirrors()
                    .at(field_span)?
                    .cols()
                    .map(|v| ValueData::Vec(self.coxeter_offset.transform_vector(v)).at(field_span))
                    .collect_vec()
                    .into(),
            ),
            "generators" => Some(
                self.generators
                    .iter()
                    .map(|g| ValueData::EuclidTransform(g.clone()).at(field_span))
                    .collect_vec()
                    .into(),
            ),
            s if s.chars().all(|c| "ox".contains(c)) => Some(
                self.coxeter_dynkin_vector(field_span, s, field_span)?
                    .into(),
            ),
            _ => None,
        })
    }

    /// Constructs a symmetry object from a Coxeter group.
    pub fn from_coxeter(coxeter: CoxeterMatrix) -> GroupResult<Self> {
        let generators = coxeter.generator_transforms()?;
        Ok(Self {
            generators: Arc::new(generators.into_vec()),
            coxeter_group: Some(Arc::new(coxeter)),
            coxeter_offset: pga::Motor::ident(0),
        })
    }

    /// Constructs a symmetry object from a list of generators.
    pub fn from_generators(generators: impl IntoIterator<Item = Motor>) -> Self {
        Self {
            generators: Arc::new(generators.into_iter().collect()),
            coxeter_group: None,
            coxeter_offset: pga::Motor::ident(0),
        }
    }

    /// Constructs a symmetry object from a Coxeter string such as `"bc3"`.
    pub fn from_cd_str(s: &str, span: Span) -> Result<Self> {
        fn split_alpha_number(s: &str) -> Option<(&str, u16, Option<u8>)> {
            let (num_index, _) = s.match_indices(char::is_numeric).next()?;
            let num_u16: u16 = s[num_index..].parse().ok()?;
            let num_u8: Option<u8> = num_u16.try_into().ok();
            Some((&s[0..num_index], num_u16, num_u8))
        }

        let coxeter = match s.to_ascii_lowercase().as_str() {
            "e6" => CoxeterMatrix::E6(),
            "e7" => CoxeterMatrix::E7(),
            "e8" => CoxeterMatrix::E8(),
            "f4" => CoxeterMatrix::F4(),
            "g2" => CoxeterMatrix::G2(),
            "h2" => CoxeterMatrix::H2(),
            "h3" => CoxeterMatrix::H3(),
            "h4" => CoxeterMatrix::H4(),
            s => match split_alpha_number(s) {
                Some(("a", _, Some(n))) => CoxeterMatrix::A(n),
                Some(("b" | "c" | "bc", _, Some(n))) => CoxeterMatrix::B(n),
                Some(("d", _, Some(n))) => CoxeterMatrix::D(n),
                Some(("i", n, _)) => CoxeterMatrix::I(n),
                _ => return Err("unknown coxeter group string".at(span)),
            }
            .at(span)?,
        };
        coxeter.try_into().at(span)
    }

    /// Constructs a symmetry object from a Schalfli symbol such as `[4, 3]`.
    pub fn from_schlafli(indices: &[u16], span: Span) -> Result<Self> {
        CoxeterMatrix::new_linear(indices)
            .at(span)?
            .try_into()
            .at(span)
    }

    /// Returns the underlying Coxeter group, or returns an error if the group
    /// was not constructed as a Coxeter group.
    pub fn as_coxeter(&self, span: Span) -> Result<&CoxeterMatrix> {
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
        let v = hypergroup::parse_dynkin_notation(self.generators.len() as u8, string)
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
        let mirror_basis = self.as_coxeter(self_span)?.mirror_basis().at(self_span)?;

        // TODO: truncate to approx nonzero
        let v_ndim = v.ndim();
        let basis_ndim = mirror_basis.ndim();
        if v_ndim > basis_ndim {
            return Err(
                format!("group has ndim {basis_ndim} but vector {v:?} has ndim {v_ndim}")
                    .at(v_span),
            );
        }

        // Transform by offset.
        // TODO: this *should* just be a matrix multiplication
        let ndim = std::cmp::max(mirror_basis.ndim(), self.coxeter_offset.ndim());
        let mirror_basis = Matrix::from_cols(
            mirror_basis
                .at_ndim(ndim)
                .cols()
                .map(|col| self.coxeter_offset.transform_vector(col)),
        );

        Ok(mirror_basis * v)
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
    pub fn orbit<T: ApproxHash + Clone + Ndim + TransformByMotor>(
        &self,
        object: T,
    ) -> Vec<(AbbrGenSeq, Motor, T)> {
        hypergroup::orbit_geometric_with_gen_seq(
            &self
                .generators
                .iter()
                .enumerate()
                .map(|(i, m)| (GenSeq::new([GeneratorId(i as u8)]), m.clone()))
                .collect_vec(),
            object,
        )
    }

    /// Returns the isometry group of the symmetry.
    pub fn isometry_group(&self) -> GroupResult<IsometryGroup> {
        IsometryGroup::from_generators("", self.generators.iter().cloned().collect())
    }

    /// Returns a list of generators of the group.
    pub fn generators(&self) -> &[Motor] {
        &self.generators
    }
}

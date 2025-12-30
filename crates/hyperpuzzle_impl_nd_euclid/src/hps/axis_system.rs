use std::fmt;
use std::sync::Arc;

use hypermath::Vector;
use hypermath::pga::Motor;
use hyperpuzzle_core::Orbit;
use hyperpuzzlescript::*;
use hypershape::AbbrGenSeq;
use itertools::Itertools;
use parking_lot::{MappedMutexGuard, MutexGuard};

use super::{HpsAxis, HpsPuzzle, HpsSymmetry, HpsTwistSystem, Names};
use crate::builder::AxisSystemBuilder;
use crate::hps::orbit_names::HpsOrbitNames;

/// HPS axis system builder.
#[derive(Clone, PartialEq, Eq)]
pub(super) struct HpsAxisSystem(pub HpsTwistSystem);
impl_simple_custom_type!(
    HpsAxisSystem = "euclid.AxisSystem",
    field_get = Self::impl_field_get,
    index_get = Self::impl_index_get,
);
impl fmt::Debug for HpsAxisSystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}
impl fmt::Display for HpsAxisSystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}(id = {:?})", self.type_name(), self.0.lock().id)
    }
}
impl HpsAxisSystem {
    fn impl_field_get(
        &self,
        _span: Span,
        (field, _field_span): Spanned<&str>,
    ) -> Result<Option<ValueData>> {
        Ok(self.axis_from_name(field)?.map(|v| v.into()))
    }
    fn impl_index_get(
        &self,
        _ctx: &mut EvalCtx<'_>,
        _span: Span,
        index: Value,
    ) -> Result<ValueData> {
        // TODO: allow indexing by numeric ID
        Ok(self.axis_from_name(index.as_ref::<str>()?)?.into())
    }

    fn axis_from_name(&self, name: &str) -> Result<Option<HpsAxis>> {
        match self.lock().names.id_from_string(name) {
            Some(id) => {
                let axes = self.clone();
                Ok(Some(HpsAxis { id, axes }))
            }
            None => Ok(None),
        }
    }
}

/// Adds the built-ins.
pub fn define_in(builtins: &mut Builtins<'_>) -> Result<()> {
    builtins.set_custom_ty::<HpsAxisSystem>()?;

    builtins.set_fns(hps_fns![
        fn add_axis(ctx: EvalCtx, vector: Vector) -> Option<HpsAxis> {
            HpsAxisSystem::get(ctx)?.add_axes(ctx, vector, None)?
        }
        fn add_axis(ctx: EvalCtx, vector: Vector, names: Names) -> Option<HpsAxis> {
            HpsAxisSystem::get(ctx)?.add_axes(ctx, vector, Some(names))?
        }
        #[kwargs(slice: bool = true)]
        fn add_axis(ctx: EvalCtx, vector: Vector, layers: Vec<Num>) -> Option<HpsAxis> {
            HpsPuzzle::get(ctx)?.add_layered_axes(ctx, vector, None, layers, slice)?
        }
        #[kwargs(slice: bool = true)]
        fn add_axis(
            ctx: EvalCtx,
            vector: Vector,
            names: Names,
            layers: Vec<Num>,
        ) -> Option<HpsAxis> {
            HpsPuzzle::get(ctx)?.add_layered_axes(ctx, vector, Some(names), layers, slice)?
        }
        #[kwargs(slice: bool = true)]
        fn add_axis(
            ctx: EvalCtx,
            names: Names,
            vector: Vector,
            layers: Vec<Num>,
        ) -> Option<HpsAxis> {
            HpsPuzzle::get(ctx)?.add_layered_axes(ctx, vector, Some(names), layers, slice)?
        }
    ])
}

impl HpsAxisSystem {
    pub fn get<'a>(ctx: &EvalCtx<'a>) -> Result<&'a Self> {
        ctx.scope.special.axes.as_ref()
    }

    pub fn lock(&self) -> MappedMutexGuard<'_, AxisSystemBuilder> {
        MutexGuard::map(self.0.lock(), |twists| &mut twists.axes)
    }

    /// Adds a symmetric set of axes.
    pub fn add_axes(
        &self,
        ctx: &mut EvalCtx<'_>,
        mut vector: Vector,
        names: Option<Names>,
    ) -> Result<Option<HpsAxis>> {
        // Canonicalize number of dimensions
        vector.resize(ctx.ndim()?);

        let span = ctx.caller_span;
        let ctx_symmetry = HpsSymmetry::get(ctx)?;
        let mut this = self.lock();

        let (gen_seqs, transforms, vectors) = match ctx_symmetry {
            Some(sym) => sym.orbit(vector).into_iter().multiunzip(),
            None => (
                vec![AbbrGenSeq::INIT],
                vec![Motor::ident(this.ndim)],
                vec![vector],
            ),
        };

        let names = match &names {
            Some(names) => names.0.to_strings(ctx, &transforms, span)?,
            None => const { &HpsOrbitNames::EMPTY }.to_strings(ctx, &[], span)?,
        }
        .chain(std::iter::repeat(None));

        // Add & name axes.
        let mut axes_list = vec![];
        for (transformed_vector, name) in std::iter::zip(&vectors, names) {
            let new_axis = this
                .add(transformed_vector.clone(), name, ctx.warnf())
                .at(span)?;
            axes_list.push(Some(new_axis));
        }
        let first_axis = axes_list.first().copied().flatten();

        if ctx_symmetry.is_some() {
            this.orbits.push(Orbit {
                elements: Arc::new(axes_list),
                generator_sequences: Arc::new(gen_seqs),
            });
        }

        Ok(first_axis.map(|id| {
            let axes = self.clone();
            HpsAxis { id, axes }
        }))
    }
}

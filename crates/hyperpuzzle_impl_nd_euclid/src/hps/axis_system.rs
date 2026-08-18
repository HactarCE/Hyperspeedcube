use std::fmt;
use std::ops::Deref;
use std::sync::Arc;

use hypergroup::AbbrGenSeq;
use hypermath::Vector;
use hypermath::pga::Motor;
use hyperpuzzle_core::util::{ExpectedAdHoc, MaybeAdHoc};
use hyperpuzzle_core::{Axis, CatalogId, IndexOutOfRange, MissingComponent, NameSpec, Orbit};
use hyperpuzzlescript::*;
use itertools::Itertools;
use parking_lot::{MappedMutexGuard, MutexGuard};

use super::{ElementNames, HpsAxis, HpsPuzzle, HpsSymmetry};
use crate::builder::{AdHocAxisSystemBuilder, TwistSystemBuilder};
use crate::components::NdEuclidAxisVectors;
use crate::hps::orbit_names::HpsOrbitNames;

/// HPS axis system builder.
#[derive(Clone)]
pub(super) struct HpsAxisSystem(pub TwistSystemBuilder);
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
        write!(f, "{}(id = {:?})", self.type_name(), self.id())
    }
}
impl PartialEq for HpsAxisSystem {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0.0, &other.0.0) {
            (MaybeAdHoc::Fixed(f1), MaybeAdHoc::Fixed(f2)) => f1.id == f2.id,
            (MaybeAdHoc::AdHoc(a1), MaybeAdHoc::AdHoc(a2)) => Arc::ptr_eq(a1, a2),
            _ => false,
        }
    }
}
impl Eq for HpsAxisSystem {}
impl HpsAxisSystem {
    fn impl_field_get(
        &self,
        _span: Span,
        (field, _field_span): Spanned<&str>,
    ) -> Result<Option<ValueData>> {
        Ok(self.axis_from_name(field).map(|v| v.into()))
    }
    fn impl_index_get(
        &self,
        _ctx: &mut EvalCtx<'_>,
        _span: Span,
        index: Value,
    ) -> Result<ValueData> {
        Ok(self.axis_from_name(index.as_ref::<str>()?).into())
    }

    pub fn axis_name(&self, axis: Axis) -> Result<Option<NameSpec>, IndexOutOfRange> {
        match &self.0.0 {
            MaybeAdHoc::Fixed(f) => Ok(Some(
                f.axes
                    .names
                    .get(axis)
                    .ok_or(IndexOutOfRange::new::<Axis>())?
                    .clone(),
            )),
            MaybeAdHoc::AdHoc(a) => Ok(a.lock().axes.names.get(axis).cloned()),
        }
    }
    fn axis_from_name(&self, name: &str) -> Option<HpsAxis> {
        let id = match &self.0.0 {
            MaybeAdHoc::Fixed(f) => f.axes.names.id_from_string(name)?,
            MaybeAdHoc::AdHoc(a) => a.lock().axes.names.id_from_string(name)?,
        };
        let axes = self.clone();
        Some(HpsAxis { id, axes })
    }
}

/// Adds the built-ins.
pub fn define_in(builtins: &mut Builtins<'_>) -> Result<()> {
    builtins.set_custom_ty::<HpsAxisSystem>()?;

    builtins.set_fns(hps_fns![
        fn add_axis(ctx: EvalCtx, vector: Vector) -> Option<HpsAxis> {
            HpsAxisSystem::get(ctx)?.add_axes(ctx, vector, None)?
        }
        fn add_axis(ctx: EvalCtx, vector: Vector, names: ElementNames) -> Option<HpsAxis> {
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
            names: ElementNames,
            layers: Vec<Num>,
        ) -> Option<HpsAxis> {
            HpsPuzzle::get(ctx)?.add_layered_axes(ctx, vector, Some(names), layers, slice)?
        }
        #[kwargs(slice: bool = true)]
        fn add_axis(
            ctx: EvalCtx,
            names: ElementNames,
            vector: Vector,
            layers: Vec<Num>,
        ) -> Option<HpsAxis> {
            HpsPuzzle::get(ctx)?.add_layered_axes(ctx, vector, Some(names), layers, slice)?
        }
    ])
}

impl HpsAxisSystem {
    pub fn get(ctx: &EvalCtx<'_>) -> Result<Self> {
        ctx.scope.special.axes.lock().as_ref().cloned()
    }

    /// Locks the ad-hoc builder if it is one, or returns an error otherwise.
    pub fn lock_ad_hoc(
        &self,
    ) -> Result<MappedMutexGuard<'_, AdHocAxisSystemBuilder>, ExpectedAdHoc> {
        Ok(MutexGuard::map(self.0.lock_ad_hoc()?, |twists| {
            &mut twists.axes
        }))
    }

    pub fn id(&self) -> CatalogId {
        match &self.0.0 {
            MaybeAdHoc::Fixed(f) => f.id.clone(),
            MaybeAdHoc::AdHoc(a) => a.lock().id.clone(),
        }
    }

    pub fn lock_vectors(
        &self,
    ) -> Result<ArcOrMutexGuard<'_, NdEuclidAxisVectors>, MissingComponent> {
        match &self.0.0 {
            // MaybeAdHoc::Fixed(f) => Ok(ArcOrMutexGuard::Arc(
            //     f.axes.components.get::<NdEuclidAxisVectors>()?,
            // )),
            MaybeAdHoc::Fixed(f) => todo!(),
            MaybeAdHoc::AdHoc(a) => Ok(ArcOrMutexGuard::MappedMutexGuard(MutexGuard::map(
                a.lock(),
                |twists| &mut twists.axes.vectors,
            ))),
        }
    }

    /// Adds a symmetric set of axes.
    pub fn add_axes(
        &self,
        ctx: &mut EvalCtx<'_>,
        vector: Vector,
        names: Option<ElementNames>,
    ) -> Result<Option<HpsAxis>> {
        let ctx_symmetry = HpsSymmetry::get(ctx)?;
        let mut this = self.lock_ad_hoc().at(ctx.caller_span)?;

        let (gen_seqs, transforms, vectors) = match ctx_symmetry {
            Some(sym) => sym
                .orbit(vector)
                .at(ctx.caller_span)?
                .into_iter()
                .multiunzip(),
            None => (
                vec![AbbrGenSeq::INIT],
                vec![Motor::ident(this.ndim())],
                vec![vector],
            ),
        };

        let names = match &names {
            Some(names) => names.0.to_opt_strings(ctx, &transforms)?,
            None => const { &HpsOrbitNames::EMPTY }.to_opt_strings(ctx, &[])?,
        }
        .chain(std::iter::repeat(None));

        // Add & name axes.
        let mut axes_list = vec![];
        for (transformed_vector, name) in std::iter::zip(&vectors, names) {
            let new_axis = this
                .add(
                    transformed_vector.clone(),
                    name.map(|s| s.into()),
                    ctx.warnf(),
                )
                .at(ctx.caller_span)?;
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

pub enum ArcOrMutexGuard<'a, T> {
    Arc(Arc<T>),
    MappedMutexGuard(MappedMutexGuard<'a, T>),
}
impl<'a, T> Deref for ArcOrMutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            ArcOrMutexGuard::Arc(r) => r,
            ArcOrMutexGuard::MappedMutexGuard(g) => g,
        }
    }
}
impl<'a, T> From<Arc<T>> for ArcOrMutexGuard<'a, T> {
    fn from(value: Arc<T>) -> Self {
        Self::Arc(value)
    }
}
impl<'a, T> From<MappedMutexGuard<'a, T>> for ArcOrMutexGuard<'a, T> {
    fn from(value: MappedMutexGuard<'a, T>) -> Self {
        Self::MappedMutexGuard(value)
    }
}

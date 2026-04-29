use std::fmt;
use std::sync::Arc;

use hypermath::{Float, Hyperplane, Vector};
use hyperpuzzle_core::prelude::*;
use hyperpuzzlescript::*;
use itertools::Itertools;
use parking_lot::{Mutex, MutexGuard};

use super::{HpsAxis, HpsRegion, HpsSymmetry, Names};
use crate::builder::*;

/// HPS puzzle builder.
#[derive(Clone)]
pub(super) struct HpsPuzzle(pub Arc<Mutex<PuzzleBuilder>>);
impl_simple_custom_type!(
    HpsPuzzle = "euclid.Puzzle",
    field_get = Self::impl_field_get,
);
impl fmt::Debug for HpsPuzzle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}
impl fmt::Display for HpsPuzzle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}(id = {:?})", self.type_name(), self.lock().meta.id)
    }
}
impl PartialEq for HpsPuzzle {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for HpsPuzzle {}
impl HpsPuzzle {
    pub fn lock(&self) -> MutexGuard<'_, PuzzleBuilder> {
        self.0.lock()
    }
}

/// Adds the built-ins.
pub fn define_in(builtins: &mut Builtins<'_>) -> Result<()> {
    builtins.set_custom_ty::<HpsPuzzle>()?;

    builtins.set_fns(hps_fns![
        #[kwargs(slice: bool = true)]
        fn add_layers(ctx: EvalCtx, (axis, axis_span): HpsAxis, layers: Vec<Num>) -> () {
            HpsPuzzle::get(ctx)?.add_layers(ctx, (axis, axis_span), layers, slice)?;
        }
    ])
}

impl HpsPuzzle {
    pub fn get<'a>(ctx: &EvalCtx<'a>) -> Result<&'a Self> {
        ctx.scope.special.puz.as_ref()
    }

    fn impl_field_get(
        &self,
        _span: Span,
        (field, _field_span): Spanned<&str>,
    ) -> Result<Option<ValueData>> {
        Ok(match field {
            "shape" => Some(self.shape().into()),
            "twists" => Some(self.twists().into()),
            "axes" => Some(self.axes().into()),
            _ => None,
        })
    }

    pub fn add_layered_axes(
        &self,
        ctx: &mut EvalCtx<'_>,
        vector: Vector,
        names: Option<Names>,
        layers: Vec<f64>,
        slice: bool,
    ) -> Result<Option<HpsAxis>> {
        let axis = self.axes().add_axes(ctx, vector, names)?;
        if let Some(axis) = axis.clone() {
            self.add_layers(ctx, (axis, ctx.caller_span), layers, slice)?;
        }
        Ok(axis)
    }

    fn add_layers(
        &self,
        ctx: &mut EvalCtx<'_>,
        (axis, axis_span): Spanned<HpsAxis>,
        layers: Vec<Float>,
        slice: bool,
    ) -> Result<()> {
        let span = ctx.caller_span;
        let ctx_symmetry = HpsSymmetry::get(ctx)?;

        let axis_vector = axis.vector().at(axis_span)?;

        let axis_vectors: Vec<Vector> = match ctx_symmetry {
            Some(sym) => sym
                .orbit(axis_vector)
                .into_iter()
                .map(|(_gen_seq, _transform, v)| v)
                .collect(),
            None => vec![axis_vector],
        };

        let axes = self.twists().axes();
        let axes_to_add_layers_to: Vec<Axis> = axis_vectors
            .iter()
            .map(|v| eyre::Ok(super::axis_from_vector(&*axes.lock_vectors()?, v)?))
            .try_collect()
            .at(span)?;

        let mut this = self.lock();

        // Add layers.
        for axis in axes_to_add_layers_to {
            this.axis_layers.extend_to_contain(axis);
            let axis_layers = &mut this.axis_layers[axis].0;
            for (&top, &bottom) in layers.iter().tuple_windows() {
                axis_layers
                    .push(AxisLayerBuilder { top, bottom })
                    .at(span)?;
            }
        }

        // Slice layers.
        if slice {
            let mut shape = this.shape.lock();
            for v in axis_vectors {
                for &distance in &layers {
                    let layer_slice_plane = Hyperplane::new(&v, distance)
                        .ok_or("bad cut plane")
                        .at(span)?;
                    shape.slice(None, layer_slice_plane, None).at(span)?;
                }
            }
        }

        Ok(())
    }

    pub fn layer_regions(
        &self,
        ctx: &mut EvalCtx<'_>,
        axis: Axis,
        layer_mask: LayerMask,
    ) -> Result<HpsRegion> {
        let span = ctx.caller_span;
        let axis_vector = self
            .axes()
            .lock_vectors()
            .at(span)?
            .vectors_by_id
            .get(axis)
            .at(span)?
            .clone();
        let this = self.lock();

        match this.plane_bounded_regions(axis, &axis_vector, layer_mask) {
            Ok(plane_bounded_regions) => Ok(HpsRegion::Or(
                plane_bounded_regions
                    .into_iter()
                    .map(|layer_region| {
                        let half_spaces = layer_region.into_iter().map(HpsRegion::HalfSpace);
                        HpsRegion::And(half_spaces.collect())
                    })
                    .collect(),
            )),
            Err(e) => {
                ctx.warn(format!("error computing region: {e:#}"));
                Ok(HpsRegion::None)
            }
        }
    }
}

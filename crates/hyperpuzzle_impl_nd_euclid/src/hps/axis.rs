use std::fmt;

use hypermath::Vector;
use hypermath::pga::Motor;
use hyperpuzzle_core::{Axis, NameSpec, util::MaybeAdHoc};
use hyperpuzzlescript::{
    Builtins, ErrorExt, EvalCtx, Result, Span, Spanned, Value, ValueData, impl_simple_custom_type,
};

use super::{HpsAxisSystem, HpsEuclidError, HpsLayerMask, HpsPuzzle};
use crate::components::NdEuclidAxisVectors;

#[derive(Clone, PartialEq, Eq)]
pub struct HpsAxis {
    pub id: Axis,
    pub axes: HpsAxisSystem,
}
impl_simple_custom_type!(
    HpsAxis = "euclid.Axis",
    field_get = Self::impl_field_get,
    index_get = Self::impl_index_get,
);
impl HpsAxis {
    fn impl_field_get(
        &self,
        self_span: Span,
        (field, _field_span): Spanned<&str>,
    ) -> Result<Option<ValueData>> {
        Ok(match field {
            "id" => Some((self.id.0 as u64).into()),
            "vec" => Some(self.vector().at(self_span)?.into()),
            "name" => Some(self.name().map(|name| name.preferred).into()),
            _ => None,
        })
    }
    fn impl_index_get(
        &self,
        ctx: &mut EvalCtx<'_>,
        _span: Span,
        index: Value,
    ) -> Result<ValueData> {
        let puzzle = HpsPuzzle::get(ctx)?;
        let HpsLayerMask(layer_mask) = index.ref_to()?;
        Ok(puzzle.layer_regions(ctx, self.id, layer_mask)?.into())
    }

    pub fn vector(&self) -> eyre::Result<Vector> {
        Ok(self
            .axes
            .lock_vectors()?
            .vectors_by_id
            .get(self.id)?
            .clone())
    }
    pub fn name(&self) -> Option<NameSpec> {
        match &self.axes.0.0 {
            MaybeAdHoc::Fixed(f) => Some(f.axes.names.get(self.id).ok()?.clone()),
            MaybeAdHoc::AdHoc(a) => Some(a.lock().axes.names.get(self.id)?.clone()),
        }
    }
}
impl fmt::Debug for HpsAxis {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "axis {}", self.id)
    }
}
impl fmt::Display for HpsAxis {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        super::fmt_puzzle_element(f, "axes", self.name(), self.id)
    }
}

/// Adds the built-ins.
pub fn define_in(builtins: &mut Builtins<'_>) -> Result<()> {
    builtins.set_custom_ty::<HpsAxis>()
}

pub(super) fn axis_from_vector(
    axes: &NdEuclidAxisVectors,
    vector: &Vector,
) -> Result<Axis, HpsEuclidError> {
    Ok(*axes
        .ids_by_vector
        .get(vector.clone())
        .ok_or_else(|| HpsEuclidError::NoAxis(vector.clone()))?)
}

pub(super) fn transform_axis(
    span: Span,
    axes: &NdEuclidAxisVectors,
    t: &Motor,
    (axis, axis_span): Spanned<Axis>,
) -> Result<(Axis, Vector)> {
    let old_vector = axes.vectors_by_id.get(axis).at(axis_span)?;
    let new_vector = t.transform(old_vector);
    Ok((axis_from_vector(axes, &new_vector).at(span)?, new_vector))
}

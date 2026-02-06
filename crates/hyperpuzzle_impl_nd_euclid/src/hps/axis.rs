use std::fmt;

use hypermath::Vector;
use hypermath::pga::Motor;
use hyperpuzzle_core::{Axis, IndexOutOfRange, NameSpec};
use hyperpuzzlescript::{
    Builtins, ErrorExt, EvalCtx, Result, Span, Spanned, Value, ValueData, impl_simple_custom_type,
};

use super::{HpsAxisSystem, HpsEuclidError, HpsLayerMask, HpsPuzzle};
use crate::builder::AxisSystemBuilder;

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

    pub fn vector(&self) -> Result<Vector, IndexOutOfRange> {
        Ok(self.axes.lock().get(self.id)?.vector().clone())
    }
    pub fn name(&self) -> Option<NameSpec> {
        Some(self.axes.lock().names.get(self.id)?.clone())
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
    axes: &AxisSystemBuilder,
    vector: &Vector,
) -> Result<Axis, HpsEuclidError> {
    axes.vector_to_id(vector)
        .ok_or_else(|| HpsEuclidError::NoAxis(vector.clone()))
}

pub(super) fn transform_axis(
    span: Span,
    axes: &AxisSystemBuilder,
    t: &Motor,
    (axis, axis_span): Spanned<Axis>,
) -> Result<Axis> {
    let old_vector = axes.get(axis).at(axis_span)?.vector();
    let new_vector = t.transform(old_vector);
    axis_from_vector(axes, &new_vector).at(span)
}

pub(super) fn axis_name(span: Span, axes: &AxisSystemBuilder, axis: Axis) -> Result<&NameSpec> {
    match axes.names.get(axis) {
        Some(name) => Ok(name),
        None => {
            let axis_vector = axes.get(axis).at(span)?.vector().clone();
            Err(HpsEuclidError::UnnamedAxis(axis, axis_vector)).at(span)
        }
    }
}

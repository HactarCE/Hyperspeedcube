use std::fmt;

use hyperpuzzle_core::{Color, NameSpec};
use hyperpuzzlescript::{Builtins, Result, Span, Spanned, ValueData, impl_simple_custom_type};

use super::HpsShape;

#[derive(Clone, PartialEq, Eq)]
pub struct HpsColor {
    pub id: Color,
    pub shape: HpsShape,
}
impl_simple_custom_type!(HpsColor = "euclid.Color", field_get = Self::impl_field_get);
impl HpsColor {
    fn impl_field_get(
        &self,
        _span: Span,
        (field, _field_span): Spanned<&str>,
    ) -> Result<Option<ValueData>> {
        Ok(match field {
            "id" => Some((self.id.0 as u64).into()),
            "name" => Some(self.name().map(|name| name.preferred).into()),
            _ => None,
        })
    }

    pub fn name(&self) -> Option<NameSpec> {
        Some(self.shape.lock().colors.names.get(self.id)?.clone())
    }
}
impl fmt::Debug for HpsColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "color {}", self.id)
    }
}
impl fmt::Display for HpsColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        super::fmt_puzzle_element(f, "colors", self.name(), self.id)
    }
}

/// Adds the built-ins.
pub fn define_in(builtins: &mut Builtins<'_>) -> Result<()> {
    builtins.set_custom_ty::<HpsColor>()
}

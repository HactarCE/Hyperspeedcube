use std::fmt;

use hypermath::IndexNewtype;
use hyperpuzzle_core::{Color, NameSpec};
use hyperpuzzlescript::{Result, Span, Spanned, ValueData, impl_simple_custom_type};

use super::HpsShape;

#[derive(Clone, PartialEq, Eq)]
pub struct HpsColor {
    pub id: Color,
    pub shape: HpsShape,
}
impl_simple_custom_type!(HpsColor = "euclid.Color", field_get = Self::field_get);
impl HpsColor {
    fn field_get(
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
        fmt_puzzle_element(f, self.name(), self.id)
    }
}

fn fmt_puzzle_element(
    f: &mut fmt::Formatter<'_>,
    name: Option<NameSpec>,
    id: impl IndexNewtype,
) -> fmt::Result {
    match name {
        Some(name) => {
            let k = hyperpuzzlescript::codegen::to_map_key(&name.preferred);
            if k.starts_with('"') {
                write!(f, "colors[{k}]")
            } else {
                write!(f, "colors.{k}")
            }
        }
        None => write!(f, "colors[{}]", id),
    }
}

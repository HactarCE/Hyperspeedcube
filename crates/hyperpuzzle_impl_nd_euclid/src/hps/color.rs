use std::fmt;

use hypermath::IndexNewtype;
use hyperpuzzle_core::{Color, NameSpec};
use hyperpuzzlescript::impl_simple_custom_type;

use super::HpsShapeBuilder;

#[derive(Clone, PartialEq, Eq)]
pub struct HpsColor {
    pub id: Color,
    pub shape: HpsShapeBuilder,
}
impl_simple_custom_type!(
    HpsColor = "euclid.Color",
    |(this, _this_span), (field, _field_span)| {
        match field {
            "id" => Some((this.id.0 as u64).into()),
            "name" => Some(this.name().map(|name| name.preferred).into()),
            _ => None,
        }
    }
);
impl HpsColor {
    pub fn name(&self) -> Option<NameSpec> {
        Some(self.shape.lock().colors.names.get(self.id)?.clone())
    }
}
impl fmt::Debug for HpsColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "axis #{}", self.id)
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

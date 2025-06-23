use std::fmt;

use hypermath::{IndexNewtype, IndexOutOfRange, Vector};
use hyperpuzzle_core::{Axis, NameSpec};
use hyperpuzzlescript::{ErrorExt, impl_simple_custom_type};

use super::HpsTwistSystem;

#[derive(Clone, PartialEq, Eq)]
pub struct HpsAxis {
    pub id: Axis,
    pub twists: HpsTwistSystem,
}
impl_simple_custom_type!(
    HpsAxis = "euclid.Axis",
    |(this, this_span), (field, _field_span)| {
        match field {
            "id" => Some((this.id.0 as u64).into()),
            "vec" => Some(this.vector().at(this_span)?.into()),
            "name" => Some(this.name().map(|name| name.preferred).into()),
            _ => None,
        }
    }
);
impl HpsAxis {
    pub fn vector(&self) -> Result<Vector, IndexOutOfRange> {
        Ok(self.twists.lock().axes.get(self.id)?.vector().clone())
    }
    pub fn name(&self) -> Option<NameSpec> {
        Some(self.twists.lock().axes.names.get(self.id)?.clone())
    }
}
impl fmt::Debug for HpsAxis {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "axis #{}", self.id)
    }
}
impl fmt::Display for HpsAxis {
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
                write!(f, "axes[{k}]")
            } else {
                write!(f, "axes.{k}")
            }
        }
        None => write!(f, "axes[{}]", id),
    }
}

use std::fmt;

use hypermath::{IndexNewtype, IndexOutOfRange, pga::Motor};
use hyperpuzzle_core::{NameSpec, Twist};
use hyperpuzzlescript::{ErrorExt, impl_simple_custom_type};

use super::{HpsAxis, HpsTwistSystem};

#[derive(Clone, PartialEq, Eq)]
pub struct HpsTwist {
    pub id: Twist,
    pub twists: HpsTwistSystem,
}
impl_simple_custom_type!(
    HpsTwist = "euclid.Twist",
    |(this, this_span), (field, _field_span)| {
        match field {
            "id" => Some((this.id.0 as u64).into()),
            "axis" => Some(this.axis().at(this_span)?.into()),
            "transform" => Some(this.transform().at(this_span)?.into()),
            "name" => Some(this.name().map(|name| name.preferred).into()),
            _ => None,
        }
    }
);
impl HpsTwist {
    pub fn axis(&self) -> Result<HpsAxis, IndexOutOfRange> {
        Ok(HpsAxis {
            id: self.twists.lock().get(self.id)?.axis,
            twists: self.twists.clone(),
        })
    }
    pub fn transform(&self) -> Result<Motor, IndexOutOfRange> {
        Ok(self.twists.lock().get(self.id)?.transform.clone())
    }
    pub fn name(&self) -> Option<NameSpec> {
        Some(self.twists.lock().names.get(self.id)?.clone())
    }
}
impl fmt::Debug for HpsTwist {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "axis #{}", self.id)
    }
}
impl fmt::Display for HpsTwist {
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
                write!(f, "twists[{k}]")
            } else {
                write!(f, "twists.{k}")
            }
        }
        None => write!(f, "twists[{}]", id),
    }
}

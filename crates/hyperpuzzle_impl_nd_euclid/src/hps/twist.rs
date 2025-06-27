use std::fmt;

use hypermath::{IndexOutOfRange, pga::Motor};
use hyperpuzzle_core::{NameSpec, Twist};
use hyperpuzzlescript::{ErrorExt, Result, Span, Spanned, ValueData, impl_simple_custom_type};

use crate::{TwistKey, builder::TwistSystemBuilder};

use super::{HpsAxis, HpsEuclidError, HpsTwistSystem};

#[derive(Clone, PartialEq, Eq)]
pub struct HpsTwist {
    pub id: Twist,
    pub twists: HpsTwistSystem,
}
impl_simple_custom_type!(HpsTwist = "euclid.Twist", field_get = Self::field_get);
impl HpsTwist {
    fn field_get(
        &self,
        span: Span,
        (field, _field_span): Spanned<&str>,
    ) -> Result<Option<ValueData>> {
        Ok(match field {
            "id" => Some((self.id.0 as u64).into()),
            "axis" => Some(self.axis().at(span)?.into()),
            "transform" => Some(self.transform().at(span)?.into()),
            "name" => Some(self.name().map(|name| name.preferred).into()),
            _ => None,
        })
    }

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
        write!(f, "twist {}", self.id)
    }
}
impl fmt::Display for HpsTwist {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        super::fmt_puzzle_element(f, "twists", self.name(), self.id)
    }
}

pub(super) fn transform_twist(
    span: Span,
    twists: &TwistSystemBuilder,
    t: &Motor,
    (twist, twist_span): Spanned<Twist>,
) -> Result<Twist> {
    let old_twist_info = twists.get(twist).at(twist_span)?;
    let new_twist_axis =
        super::transform_axis(span, &twists.axes, &t, (old_twist_info.axis, twist_span))?;
    let new_twist_transform = t.transform(&old_twist_info.transform);
    let new_twist_key = TwistKey::new(new_twist_axis, &new_twist_transform)
        .ok_or(HpsEuclidError::BadTwistTransform)
        .at(span)?;
    twists
        .key_to_id(&new_twist_key)
        .ok_or(HpsEuclidError::NoTwist(new_twist_key))
        .at(span)
}

pub(super) fn twist_name(
    span: Span,
    twists: &TwistSystemBuilder,
    twist: Twist,
) -> Result<&NameSpec> {
    match twists.names.get(twist) {
        Some(name) => Ok(name),
        None => {
            let twist_key = twists.get(twist).at(span)?.key().at(span)?;
            Err(HpsEuclidError::UnnamedTwist(twist, twist_key)).at(span)
        }
    }
}

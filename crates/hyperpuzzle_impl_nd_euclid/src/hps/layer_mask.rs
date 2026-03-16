use hyperpuzzle_core::{Layer, LayerMask};
use hyperpuzzlescript::{
    Error, FromValue, FromValueRef, List, Result, Type, TypeOf, Value, impl_ty,
};

pub struct HpsLayerMask(pub LayerMask);
impl_ty!(HpsLayerMask = Type::Null | Type::Nat | Type::List(Some(Box::new(Type::Nat))));
impl<'a> FromValueRef<'a> for HpsLayerMask {
    fn from_value_ref(value: &'a Value) -> Result<Self> {
        if value.is_null() {
            Ok(Self(LayerMask::EMPTY))
        } else if let Ok(i) = value.ref_to::<u16>() {
            match Layer::new(i) {
                Some(layer) => Ok(HpsLayerMask(LayerMask::from_layer(layer))),
                None => Err(Error::IndexOutOfBounds {
                    got: i as i64,
                    bounds: Some((Layer::MIN.to_i16() as i64, Layer::MAX.to_i16() as i64)),
                }
                .at(value.span)),
            }
        } else if value.is::<List>() {
            Ok(Self(
                value
                    .ref_to::<Vec<u16>>()?
                    .into_iter()
                    .filter_map(Layer::new)
                    .collect(),
            ))
        } else {
            Err(value.type_error(Self::hps_ty()))
        }
    }
}
impl FromValue for HpsLayerMask {
    fn from_value(value: Value) -> Result<Self> {
        Self::from_value_ref(&value)
    }
}

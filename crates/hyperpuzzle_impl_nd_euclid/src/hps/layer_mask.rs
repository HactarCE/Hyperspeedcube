use hyperpuzzle_core::{Layer, LayerMask, LayerMaskUint};
use hyperpuzzlescript::{
    Error, FromValue, FromValueRef, List, Result, Span, Spanned, Type, TypeOf, Value, impl_ty,
};
use itertools::Itertools;

pub struct HpsLayerMask(pub LayerMask);
impl_ty!(HpsLayerMask = Type::Null | Type::Nat | Type::List(Some(Box::new(Type::Nat))));
impl<'a> FromValueRef<'a> for HpsLayerMask {
    fn from_value_ref(value: &'a Value) -> Result<Self> {
        if value.is_null() {
            Ok(Self(LayerMask::all_layers(LayerMaskUint::BITS as u8)))
        } else if let Ok(i) = value.ref_to::<u8>() {
            Ok(Self(LayerMask::from(layer_from_num(value.span, i)?)))
        } else if value.is::<List>() {
            value
                .ref_to::<Vec<Spanned<u8>>>()?
                .into_iter()
                .map(|(i, span)| Ok(LayerMask::from(layer_from_num(span, i)?)))
                .fold_ok(LayerMask::EMPTY, |a, b| a | b)
                .map(Self)
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

fn layer_from_num(span: Span, i: u8) -> Result<Layer> {
    i.checked_sub(1)
        .filter(|&i| i < LayerMaskUint::BITS as u8)
        .map(Layer)
        .ok_or_else(|| {
            Error::IndexOutOfBounds {
                got: i as i64,
                bounds: Some((1, LayerMaskUint::BITS as i64 - 1)),
            }
            .at(span)
        })
}

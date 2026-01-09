use std::sync::Arc;

use ecow::EcoString;
use hypermath::Vector;
use itertools::Itertools;

use crate::{
    EmptyList, FnValue, Key, List, ListOf, Map, NonEmptyList, NonEmptyListOf, NonEmptyVec, Scope,
    Type, Value, ValueData,
};

macro_rules! impl_to_value_data {
    ($ty:ty, $value:pat => $ret:expr) => {
        impl From<$ty> for ValueData {
            fn from($value: $ty) -> Self {
                $ret
            }
        }
    };
}

// Meta
impl From<Value> for ValueData {
    fn from(value: Value) -> Self {
        value.data
    }
}
impl<V: Into<ValueData>> From<Option<V>> for ValueData {
    fn from(value: Option<V>) -> Self {
        match value {
            Some(v) => v.into(),
            None => ValueData::Null,
        }
    }
}
impl From<&Scope> for ValueData {
    fn from(value: &Scope) -> Self {
        Self::Map(Arc::new(
            value
                .names
                .lock()
                .iter()
                .map(|(name, value)| (name.clone(), value.clone()))
                .sorted_by(|(name1, _), (name2, _)| name1.cmp(name2))
                .collect(),
        ))
    }
}

// Miscellaneous values
impl_to_value_data!((), () => ValueData::Null);
impl_to_value_data!(bool, b => ValueData::Bool(b));
impl_to_value_data!(Vector, v => ValueData::Vec(v));
impl_to_value_data!(Type, t => ValueData::Type(t));
impl_to_value_data!(FnValue, f => ValueData::Fn(Arc::new(f)));
impl_to_value_data!(Arc<FnValue>, f => ValueData::Fn(f));

// Numbers
impl_to_value_data!(f64, n => ValueData::Num(n));
impl_to_value_data!(i64, n => ValueData::Num(n as f64));
impl_to_value_data!(u64, n => ValueData::Num(n as f64));
impl_to_value_data!(u8, n => ValueData::Num(n as f64));
impl_to_value_data!(usize, n => ValueData::Num(n as f64));

// Strings
impl_to_value_data!(EcoString, s => ValueData::Str(s));
impl_to_value_data!(&str, s => ValueData::Str(s.into()));
impl_to_value_data!(String, s => ValueData::Str(s.into()));
impl_to_value_data!(char, c => ValueData::Str(c.into()));
impl_to_value_data!(Key, s => ValueData::Str(s.as_str().into()));

// Collections
impl FromIterator<Value> for ValueData {
    fn from_iter<T: IntoIterator<Item = Value>>(iter: T) -> Self {
        Self::List(Arc::new(iter.into_iter().collect()))
    }
}
impl<K: Into<Key>> FromIterator<(K, Value)> for ValueData {
    fn from_iter<T: IntoIterator<Item = (K, Value)>>(iter: T) -> Self {
        Self::Map(Arc::new(
            iter.into_iter().map(|(k, v)| (k.into(), v)).collect(),
        ))
    }
}
impl<V: Into<ValueData>> From<ListOf<V>> for ValueData {
    fn from(value: ListOf<V>) -> Self {
        Self::List(Arc::new(
            value
                .into_iter()
                .map(|(data, span)| Value {
                    data: data.into(),
                    span,
                })
                .collect(),
        ))
    }
}
impl<V: Into<ValueData>> From<NonEmptyListOf<V>> for ValueData {
    fn from(value: NonEmptyListOf<V>) -> Self {
        ValueData::from(value.0)
    }
}
impl_to_value_data!(List, values => ValueData::List(Arc::new(values)));
impl_to_value_data!(NonEmptyList, NonEmptyVec(values) => ValueData::from(values));
impl_to_value_data!(EmptyList, EmptyList => ValueData::List(Arc::new(vec![])));
impl_to_value_data!(Arc<List>, values => ValueData::List(values));
impl_to_value_data!(Map, values => ValueData::Map(Arc::new(values)));
impl_to_value_data!(Arc<Map>, values => ValueData::Map(values));

// hypermath
impl_to_value_data!(hypermath::Point, p => ValueData::EuclidPoint(p));
impl_to_value_data!(hypermath::pga::Motor, t => ValueData::EuclidTransform(t));
impl_to_value_data!(hypermath::Hyperplane, h => ValueData::EuclidPlane(Box::new(h)));

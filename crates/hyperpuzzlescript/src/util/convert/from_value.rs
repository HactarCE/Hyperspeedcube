use std::sync::Arc;

use ecow::EcoString;
use hypermath::Vector;
use indexmap::IndexMap;

use crate::{Error, FnValue, Key, List, Map, Result, Spanned, Type, TypeOf, Value, ValueData};

/// Trait for values that can be acquired from an owned [`Value`].
pub trait FromValue: Sized + TypeOf {
    /// Returns `Self` from `Value`.
    fn from_value(value: Value) -> Result<Self>;
}
impl FromValue for Value {
    fn from_value(value: Value) -> Result<Self> {
        Ok(value)
    }
}
impl FromValue for ValueData {
    fn from_value(value: Value) -> Result<Self> {
        Ok(value.data)
    }
}
impl<T: FromValue> FromValue for Spanned<T> {
    fn from_value(value: Value) -> Result<Self> {
        let span = value.span;
        Ok((T::from_value(value)?, span))
    }
}
impl<T: FromValue> FromValue for Option<T> {
    fn from_value(value: Value) -> Result<Self> {
        match value.is_null() {
            true => Ok(None),
            false => match T::from_value(value) {
                Ok(v) => Ok(Some(v)),
                Err(e) => Err(e.with_expected_type(Self::hps_ty())),
            },
        }
    }
}

/// Trait for values that can be acquired from a reference [`&Value`][Value].
pub trait FromValueRef<'a>: Sized + TypeOf {
    /// Returns `Self` from `&Value`.
    fn from_value_ref(value: &'a Value) -> Result<Self>;
}
impl<'a, T: FromValueRef<'a>> FromValueRef<'a> for Spanned<T> {
    fn from_value_ref(value: &'a Value) -> Result<Self> {
        Ok((T::from_value_ref(value)?, value.span))
    }
}
impl<'a, T: FromValueRef<'a>> FromValueRef<'a> for Option<T> {
    fn from_value_ref(value: &'a Value) -> Result<Self> {
        match value.is_null() {
            true => Ok(None),
            false => match T::from_value_ref(value) {
                Ok(v) => Ok(Some(v)),
                Err(e) => Err(e.with_expected_type(Self::hps_ty())),
            },
        }
    }
}

macro_rules! impl_from_value_arc {
    ($ty:ty $(= $hps_ty:expr)?, $variant:ident $contents:tt => $ret:expr) => {
        $( impl_ty!($ty = $hps_ty); )?
        impl_from_value_ref!(for<'a> &'a $ty, $variant $contents => $ret);
        impl_from_value_borrowable!(Arc<$ty>, $variant $contents => $ret);
    };
}
macro_rules! impl_from_value_borrowable {
    ($ty:ty $(= $hps_ty:expr)?, $($rest:tt)*) => {
        $( impl_ty!($ty = $hps_ty); )?
        impl_from_value_ref!(for<'a> &'a $ty, $($rest)*);
        impl_from_value!($ty, $($rest)*);
    };
}
macro_rules! impl_from_value_copyable {
    ($ty:ty $(= $hps_ty:expr)?, $($rest:tt)*) => {
        $( impl_ty!($ty = $hps_ty); )?
        impl_from_value_ref!(for<'a> $ty, $($rest)*);
        impl FromValue for $ty {
            fn from_value(value: Value) -> Result<Self> {
                <Self>::from_value_ref(&value)
            }
        }
    };
}

/// `impl FromValue for $ty`
macro_rules! impl_from_value {
    ($ty:ty $(= $hps_ty:expr)?, $variant:ident $contents:tt => $ret:expr) => {
        $( impl_ty!($ty = $hps_ty); )?
        impl FromValue for $ty {
            fn from_value(value: Value) -> Result<Self> {
                match value.data {
                    ValueData::$variant $contents => {
                        let result: Result<$ty, Error> = $ret;
                        result.map_err(|e: Error| e.at(value.span))
                    },
                    other => Err(other.at(value.span).type_error(Self::hps_ty())),
                }
            }
        }
    };
}
/// `impl FromValueRef for $ty`
macro_rules! impl_from_value_ref {
    (for<$lt:lifetime> $ty:ty, $variant:ident $contents:tt => $ret:expr) => {
        impl<$lt> FromValueRef<$lt> for $ty
        where
            $ty: $lt
        {
            fn from_value_ref(value: &$lt Value) -> Result<Self> {
                match &value.data {
                    ValueData::$variant $contents => {
                        let result: Result<$ty, Error> = $ret;
                        result.map_err(|e: Error| e.at(value.span))
                    },
                    _ => Err(value.type_error(Self::hps_ty())),
                }
            }
        }
    };
    ($ty:ty, $variant:ident $contents:tt => $ret:expr) => {
        impl_from_value_ref!(for<'a> $ty, $variant $contents => $ret);
    };
}

// Miscellaneous values
impl_from_value_copyable!(() = Type::Null, Null { .. } => Ok(()));
impl_from_value_copyable!(bool = Type::Bool, Bool(b) => Ok(*b));
impl_from_value_borrowable!(Vector = Type::Vec, Vec(v) => Ok(v));
impl_from_value_borrowable!(Type = Type::Type, Type(t) => Ok(t));
impl_from_value_arc!(FnValue = Type::Fn, Fn(f) => Ok(f));

// Numbers
impl_from_value_copyable!(f64 = Type::Num, Num(n) => Ok(*n));
impl_from_value_copyable!(i64 = Type::Int, Num(n) => {
    hypermath::to_approx_integer(*n).ok_or(Error::ExpectedInteger(*n))
});
impl_from_value_copyable!(u64 = Type::Nat, Num(n) => {
    hypermath::to_approx_unsigned_integer(*n).ok_or(Error::ExpectedNonnegativeInteger(*n))
});
impl_from_value_copyable!(u32 = Type::Nat, Num(n) => {
    hypermath::to_approx_unsigned_integer(*n)
        .and_then(|n| n.try_into().ok())
        .ok_or(Error::ExpectedNonnegativeInteger(*n))
});
impl_from_value_copyable!(u8 = Type::Nat, Num(n) => {
    hypermath::to_approx_integer(*n)
        .and_then(|n| n.try_into().ok())
        .ok_or(Error::ExpectedSmallNonnegativeInteger(*n))
});
impl_from_value_copyable!(usize = Type::Nat, Num(n) => {
    hypermath::to_approx_unsigned_integer(*n)
        .map(|n| n.try_into().unwrap_or(usize::MAX))
        .ok_or(Error::ExpectedNonnegativeInteger(*n))
});

// Strings
impl_from_value_borrowable!(EcoString = Type::Str, Str(s) => Ok(s));
impl_ty!(str = Type::Str);
impl_from_value_ref!(for<'a> &'a str, Str(s) => Ok(s));
impl_from_value!(String = Type::Str, Str(s) => Ok(s.into()));
impl_ty!(char = Type::Str);
impl_from_value!(Key = Type::Str, Str(s) => Ok(s.as_str().into()));

// Collections
impl<T: TypeOf> TypeOf for Vec<T> {
    fn hps_ty() -> Type {
        let inner_type = Some(T::hps_ty()).filter(|t| *t != Type::Any);
        Type::List(inner_type.map(Box::new))
    }
}
impl<'a, T: FromValueRef<'a>> FromValueRef<'a> for Vec<T> {
    fn from_value_ref(value: &'a Value) -> Result<Self> {
        value
            .as_ref::<Arc<Vec<_>>>()
            .map_err(|e| e.with_expected_type(Self::hps_ty()))?
            .iter()
            .map(T::from_value_ref)
            .collect()
    }
}
impl<T> TypeOf for IndexMap<Key, T> {
    fn hps_ty() -> Type {
        Type::Map
    }
}
impl<'a, T: FromValueRef<'a>> FromValueRef<'a> for IndexMap<Key, T> {
    fn from_value_ref(value: &'a Value) -> Result<Self> {
        value
            .as_ref::<Arc<Map>>()?
            .iter()
            .map(|(k, v)| Ok((k.clone(), T::from_value_ref(v)?)))
            .collect()
    }
}
impl_from_value_arc!(List, List(l) => Ok(l));
impl_from_value_arc!(Map, Map(m) => Ok(m));
impl<T: FromValue> FromValue for Vec<T> {
    fn from_value(value: Value) -> Result<Self> {
        value
            .unwrap_or_clone_arc::<Vec<_>>()
            .map_err(|e| e.with_expected_type(Self::hps_ty()))?
            .into_iter()
            .map(T::from_value)
            .collect()
    }
}

// hypermath
impl_from_value_borrowable!(hypermath::Point = Type::EuclidPoint, EuclidPoint(p) => Ok(p));
impl_from_value_borrowable!(hypermath::pga::Motor = Type::EuclidTransform, EuclidTransform(t) => Ok(t));
impl_from_value_ref!(for<'a> &'a hypermath::Hyperplane, EuclidPlane(h) => Ok(h));
impl_from_value!(hypermath::Hyperplane = Type::EuclidPlane, EuclidPlane(h) => Ok(*h));
impl_ty!(hypermath::pga::Blade = Type::Vec | Type::EuclidPoint | Type::EuclidBlade);
impl<'a> FromValueRef<'a> for hypermath::pga::Blade {
    fn from_value_ref(value: &'a Value) -> Result<Self> {
        match &value.data {
            ValueData::Vec(v) => Ok(hypermath::pga::Blade::from_vector(v)),
            ValueData::EuclidPoint(p) => Ok(hypermath::pga::Blade::from_point(p)),
            ValueData::EuclidBlade(b) => Ok(b.clone()),
            _ => Err(value.type_error(Self::hps_ty())),
        }
    }
}
impl FromValue for hypermath::pga::Blade {
    fn from_value(value: Value) -> Result<Self> {
        match value.data {
            ValueData::Vec(v) => Ok(hypermath::pga::Blade::from_vector(v)),
            ValueData::EuclidPoint(p) => Ok(hypermath::pga::Blade::from_point(&p)),
            ValueData::EuclidBlade(b) => Ok(b),
            _ => Err(value.type_error(Self::hps_ty())),
        }
    }
}

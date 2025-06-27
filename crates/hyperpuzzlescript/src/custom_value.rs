use std::any::Any;

use hyperpuzzle_core::{box_dyn_wrapper_struct, impl_dyn_clone};

use crate::{
    Error, FromValue, FromValueRef, Result, Span, Spanned, Type, TypeOf, Value, ValueData,
};

/// Implements a custom type
#[macro_export]
macro_rules! impl_simple_custom_type {
    ($ty:ty = $name:literal $(, $method_name:ident = $impl_name:expr)* $(,)?) => {
        $crate::impl_ty!($ty = $name);
        impl $crate::CustomValue for $ty {
            fn type_name(&self) -> &'static str {
                $name
            }

            fn clone_dyn(&self) -> $crate::BoxDynValue {
                self.clone().into()
            }

            fn fmt(&self, f: &mut std::fmt::Formatter<'_>, is_repr: bool) -> std::fmt::Result {
                match is_repr {
                    false => write!(f, "{self:?}"),
                    true => write!(f, "{self}"),
                }
            }

            $(
                $crate::impl_simple_custom_type!(@method $method_name = $impl_name);
            )*

            fn eq(&self, other: &$crate::BoxDynValue) -> Option<bool> {
                $crate::TryEq::try_eq(self, other.downcast_ref::<Self>()?)
            }
        }
    };
    (@method field_get = $method_impl:expr) => {
        fn field_get(&self, self_span: Span, field: Spanned<&str>) -> Result<Option<ValueData>> {
            $method_impl(self, self_span, field)
        }
    };
    (@method index_get = $method_impl:expr) => {
        fn index_get(&self, self_span: Span, index: &Value) -> Result<ValueData> {
            $method_impl(self, self_span, index)
        }
    };
}

/// Trait for values that may or may not be possible to compare for equality.
pub trait TryEq {
    /// Returns whether two values are equal, or returns `None` if they cannot
    /// be compared.
    fn try_eq(&self, other: &Self) -> Option<bool>;
}
impl<T: PartialEq> TryEq for T {
    fn try_eq(&self, other: &Self) -> Option<bool> {
        Some(self == other)
    }
}

box_dyn_wrapper_struct! {
    /// Wrapper around `Box<dyn CustomValue>` that can be downcast to a concrete
    /// puzzle state type. It also implements `Clone` for convenience.
    pub struct BoxDynValue(Box<dyn CustomValue>);
}
impl_dyn_clone!(for BoxDynValue);

/// Trait that downstream types can implement to be representable by
/// [`ValueData::Custom`].
///
/// You may also want to implement [`TypeOf`]:
///
/// ```
/// # use hyperpuzzlescript::*;
/// struct MyCustomType;
/// impl_ty!(MyCustomType = "MyCustomType");
/// impl CustomValue for MyCustomType {
///     // ...
/// # fn type_name(&self) -> &'static str { unimplemented!() }
/// # fn clone_dyn(&self) -> BoxDynValue { unimplemented!() }
/// # fn fmt(&self, f: &mut std::fmt::Formatter<'_>, is_debug: bool) -> std::fmt::Result { unimplemented!() }
/// # fn field_get(&self, self_span: Span, field: &str, field_span: Span) -> Result<Option<ValueData>> { unimplemented!() }
/// }
/// ```
pub trait CustomValue: Any + Send + Sync {
    /// Returns a user-friendly name for the type.
    ///
    /// This will not automatically be added to the global scope to refer to the
    /// type.
    fn type_name(&self) -> &'static str;

    /// Clones the value.
    fn clone_dyn(&self) -> BoxDynValue;

    /// Formats the value.
    ///
    /// If `repr` is `true`, then this should produce a string representation of
    /// the value that the user could have created themself. If this is
    /// impossible or very ugly, then it should produce a string representation
    /// captures as much detail about the value as is reasonable.
    ///
    /// If `repr` is `false`, then this should produce the most generally-useful
    /// string representation of the value.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>, is_repr: bool) -> std::fmt::Result;

    /// Converts to a [`Value`].
    fn at(self, span: Span) -> Value
    where
        Self: Sized,
    {
        ValueData::Custom(BoxDynValue::new(self)).at(span)
    }

    /// Returns a field of the type.
    fn field_get(&self, _self_span: Span, _field: Spanned<&str>) -> Result<Option<ValueData>> {
        Ok(None)
    }

    /// Indexes the type.
    fn index_get(&self, self_span: Span, _index: &Value) -> Result<ValueData> {
        Err(Error::CannotIndex(Type::Custom(self.type_name())).at(self_span))
    }

    /// Returns whether two values are equal, or returns `None` if they cannot
    /// be compared.
    fn eq(&self, other: &BoxDynValue) -> Option<bool>;
}

impl<'a, T: CustomValue + TypeOf> FromValueRef<'a> for &'a T {
    fn from_value_ref(value: &'a Value) -> Result<Self> {
        match &value.data {
            ValueData::Custom(box_dyn_value) => box_dyn_value.downcast_ref(),
            _ => None,
        }
        .ok_or_else(|| value.type_error(T::hps_ty()))
    }
}

impl<T: CustomValue + TypeOf> FromValue for T {
    fn from_value(value: Value) -> Result<Self> {
        match &value.data {
            ValueData::Custom(box_dyn_value) if box_dyn_value.downcast_ref::<T>().is_some() => {
                let ValueData::Custom(box_dyn_value) = value.data else {
                    unreachable!()
                };
                Ok(*box_dyn_value
                    .downcast()
                    .ok_or(Error::Internal("downcast failed").at(value.span))?)
            }
            _ => Err(value.type_error(T::hps_ty())),
        }
    }
}

impl<T: CustomValue> From<T> for ValueData {
    fn from(value: T) -> Self {
        ValueData::Custom(BoxDynValue::new(value))
    }
}

use std::sync::Arc;

use arcstr::Substr;

#[macro_use]
mod type_of;
#[macro_use]
mod from_value;
#[macro_use]
mod to_value;

pub use from_value::{FromValue, FromValueRef};
pub use type_of::{TypeOf, hps_ty};

use crate::{FnValue, Map, Result, Span, Value, ValueData};

impl Value {
    /// Takes ownership of the value and returns `T` or a type error.
    ///
    /// Use this for non-`Copy` types that are contained directly in [`Value`].
    pub fn to<T: FromValue>(self) -> Result<T> {
        T::from_value(self)
    }
    /// Takes a reference to the value and returns `T` or a type error.
    ///
    /// Use this for `Copy` types. For references, see [`Value::as_ref()`].
    pub fn ref_to<'a, T: FromValueRef<'a>>(&'a self) -> Result<T> {
        T::from_value_ref(self)
    }
    /// Takes a reference to the value and returns `&T` or a type error.
    ///
    /// Use this for references to non-`Copy` types.
    /// [`Value::as_ref::<T>()`][Self::as_ref] is equivalent to
    /// [`Value::ref_to::<&T>()`][Self::ref_to].
    pub fn as_ref<'a, T: ?Sized>(&'a self) -> Result<&'a T>
    where
        &'a T: FromValueRef<'a>,
    {
        self.ref_to()
    }
    /// Takes a reference to `&T` from the value and clones it, returning `T` or
    /// a type error.
    ///
    /// Use this for non-`Copy` types that you want to clone without taking
    /// ownership of the original value.
    pub fn clone_to<'a, T: 'a + Clone>(&'a self) -> Result<T>
    where
        &'a T: FromValueRef<'a>,
    {
        self.as_ref::<T>().cloned()
    }

    /// Takes ownership of the value, gets `Arc<T>`, and unwraps or clones the
    /// containing value. Returns `T` or a type error.
    ///
    /// Use this for types contained in an [`Arc`] inside [`Value`] when you
    /// need ownership of the contents.
    pub fn unwrap_or_clone_arc<T: Clone>(self) -> Result<T>
    where
        Arc<T>: FromValue,
    {
        self.to::<Arc<T>>().map(Arc::unwrap_or_clone)
    }

    /// Returns the function. If the value wasn't a function before, replaces it
    /// with a new function with the given name.
    pub fn as_func_mut(&mut self, span: Span, name: Option<Substr>) -> &mut FnValue {
        if !self.is_func() {
            *self = ValueData::Fn(Arc::new(FnValue::new(name))).at(span);
        }
        match &mut self.data {
            ValueData::Fn(f) => Arc::make_mut(f),
            _ => unreachable!(),
        }
    }

    /// Returns the map. If the value wasn't a map before, replaces it with a
    /// new map.
    pub fn as_map_mut(&mut self, span: Span) -> &mut Map {
        if !matches!(self.data, ValueData::Map(_)) {
            *self = ValueData::Map(Arc::new(Map::new())).at(span);
        }
        match &mut self.data {
            ValueData::Map(m) => Arc::make_mut(m),
            _ => unreachable!(),
        }
    }

    /// Returns whether the value is the given type.
    pub fn is<T: ?Sized>(&self) -> bool
    where
        for<'a> &'a T: FromValueRef<'a>,
    {
        self.as_ref::<T>().is_ok()
    }
}

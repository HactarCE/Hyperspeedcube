use std::ops::Deref;

use hypermath::Vector;
use hypermath::pga::Blade;
use rhai::Dynamic;

use crate::{ConvertError, Ctx, InKey, Point, Result};

/// Converts a Rhai value to `T` or returns an error.
pub fn from_rhai<T: FromRhai>(ctx: &Ctx<'_>, value: Dynamic) -> Result<T, ConvertError> {
    T::try_from_rhai(ctx, value)
}

/// Converts an optional Rhai value to `T` or returns an error.
pub fn from_rhai_opt<T: FromRhai>(
    ctx: &Ctx<'_>,
    value: Option<Dynamic>,
) -> Result<T, ConvertError> {
    T::try_from_rhai_opt(ctx, value)
}

/// Trait for converting [`rhai::Dynamic`] into specific types with good error
/// messages.
pub trait FromRhai: 'static + Sized {
    /// Returns a user-friendly constant string describing the expected type.
    fn expected_string() -> String;

    /// Converts [`Dynamic`] to `T` or returns an error.
    fn try_from_rhai(ctx: &Ctx<'_>, value: Dynamic) -> Result<Self, ConvertError> {
        value
            .try_cast_result()
            .map_err(|v| ConvertError::new::<Self>(ctx, Some(&v)))
    }

    /// Converts [`None`] to `T` or returns an error.
    fn try_from_none(ctx: &Ctx<'_>) -> Result<Self, ConvertError> {
        Err(ConvertError::new::<Self>(ctx, None))
    }

    /// Converts an `Option<Dynamic>` to `T` or returns an error.
    fn try_from_rhai_opt(ctx: &Ctx<'_>, value: Option<Dynamic>) -> Result<Self, ConvertError> {
        match value {
            Some(inner) => Self::try_from_rhai(ctx, inner),
            None => Self::try_from_none(ctx),
        }
    }
}

impl<T: FromRhai> FromRhai for Option<T> {
    fn expected_string() -> String {
        format!("optional {}", T::expected_string())
    }

    fn try_from_rhai(ctx: &Ctx<'_>, value: Dynamic) -> Result<Self, ConvertError> {
        from_rhai::<T>(ctx, value).map(Some)
    }

    fn try_from_none(_ctx: &Ctx<'_>) -> Result<Self, ConvertError> {
        Ok(None)
    }
}

impl<T: FromRhai> FromRhai for Vec<T> {
    fn expected_string() -> String {
        format!("array of {}", T::expected_string())
    }

    fn try_from_rhai(ctx: &Ctx<'_>, value: Dynamic) -> Result<Self, ConvertError> {
        if !value.is_array() {
            return Err(ConvertError::new::<Self>(ctx, Some(&value)));
        }
        value
            .cast::<rhai::Array>()
            .into_iter()
            .map(|elem| from_rhai(ctx, elem))
            .collect::<Result<Vec<T>, ConvertError>>()
            .in_structure("array")
    }
}

macro_rules! impl_from_rhai {
    ($type:ty, $name:expr $(, $impl:expr)?) => {
        impl FromRhai for $type {
            fn expected_string() -> String {
                $name.to_owned()
            }

            $(
                fn try_from_rhai(ctx: &Ctx<'_>, value: Dynamic) -> Result<Self, ConvertError> {
                    let f: fn(&Ctx<'_>, Dynamic) -> Result<Self, ConvertError> = $impl;
                    f(ctx, value)
                }
            )?
        }
    };
}

// Built-in types
impl_from_rhai!(Dynamic, "value");
impl_from_rhai!(rhai::Map, "map");
impl_from_rhai!(String, "string");
impl_from_rhai!(char, "char");
impl_from_rhai!(f64, "number", |ctx, value| {
    None.or_else(|| value.as_float().ok())
        .or_else(|| value.as_int().ok().map(|i| i as f64))
        .ok_or_else(|| ConvertError::new::<Self>(ctx, Some(&value)))
});

// Math types
impl_from_rhai!(Vector, "vector");
impl_from_rhai!(Point, "point");
impl_from_rhai!(Blade, "vector, point, or PGA blade", |ctx, value| {
    Err(value)
        .or_else(|val| val.try_cast_result::<Blade>())
        .or_else(|v| v.try_cast_result().map(|v: Vector| Blade::from_vector(v)))
        .or_else(|v| v.try_cast_result().map(|p: Point| Blade::from_vector(p.0)))
        .map_err(|v| ConvertError::new::<Self>(ctx, Some(&v)))
});

pub struct OptVecOrSingle<T>(pub Vec<T>);
impl<T: FromRhai> FromRhai for OptVecOrSingle<T> {
    fn expected_string() -> String {
        format!("{}, or array of them", T::expected_string())
    }

    fn try_from_rhai(ctx: &Ctx<'_>, value: Dynamic) -> Result<Self, ConvertError> {
        if value.is_array() {
            from_rhai::<Vec<T>>(ctx, value).map(Self)
        } else {
            from_rhai::<T>(ctx, value).map(|out| Self(vec![out]))
        }
        .map_err(|e| ConvertError {
            expected: Self::expected_string(),
            ..e
        })
    }

    fn try_from_none(_ctx: &Ctx<'_>) -> Result<Self, ConvertError> {
        Ok(Self(vec![]))
    }
}
impl<T> OptVecOrSingle<T> {
    pub fn into_vec(self) -> Vec<T> {
        self.0
    }
}
impl<T> IntoIterator for OptVecOrSingle<T> {
    type Item = T;

    type IntoIter = <Vec<T> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.into_vec().into_iter()
    }
}
impl<T> Deref for OptVecOrSingle<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

use std::ops::Deref;

use hypermath::pga::{Blade, Motor};
use hypermath::{Point, Vector};
use rhai::{Dynamic, FnPtr};

use crate::package::{RhaiAxis, RhaiColor, RhaiTwist};
use crate::{ConvertError, InKey, Result, RhaiCtx};

/// Converts a Rhai value to `T` or returns an error.
pub fn from_rhai<T: FromRhai>(ctx: impl RhaiCtx, value: Dynamic) -> Result<T, ConvertError> {
    T::try_from_rhai(ctx, value)
}

/// Converts an optional Rhai value to `T` or returns an error.
pub fn from_rhai_opt<T: FromRhai>(
    ctx: impl RhaiCtx,
    value: Option<Dynamic>,
) -> Result<T, ConvertError> {
    T::try_from_rhai_opt(ctx, value)
}

pub fn from_rhai_array<T: FromRhai>(
    ctx: impl RhaiCtx,
    array: rhai::Array,
) -> Result<Vec<T>, ConvertError> {
    from_rhai(ctx, Dynamic::from_array(array))
}

/// Trait for converting [`rhai::Dynamic`] into specific types with good error
/// messages.
pub trait FromRhai: 'static + Sized {
    /// Returns a user-friendly constant string describing the expected type.
    fn expected_string() -> String;

    /// Converts [`Dynamic`] to `T` or returns an error.
    fn try_from_rhai(ctx: impl RhaiCtx, value: Dynamic) -> Result<Self, ConvertError> {
        value
            .try_cast_result()
            .map_err(|v| ConvertError::new::<Self>(ctx, Some(&v)))
    }

    /// Converts [`None`] to `T` or returns an error.
    fn try_from_none(ctx: impl RhaiCtx) -> Result<Self, ConvertError> {
        Err(ConvertError::new::<Self>(ctx, None))
    }

    /// Converts an `Option<Dynamic>` to `T` or returns an error.
    fn try_from_rhai_opt(ctx: impl RhaiCtx, value: Option<Dynamic>) -> Result<Self, ConvertError> {
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

    fn try_from_rhai(ctx: impl RhaiCtx, value: Dynamic) -> Result<Self, ConvertError> {
        from_rhai::<T>(ctx, value).map(Some)
    }

    fn try_from_none(_ctx: impl RhaiCtx) -> Result<Self, ConvertError> {
        Ok(None)
    }
}

impl<T: FromRhai> FromRhai for Vec<T> {
    fn expected_string() -> String {
        format!("array of {}", T::expected_string())
    }

    fn try_from_rhai(mut ctx: impl RhaiCtx, value: Dynamic) -> Result<Self, ConvertError> {
        if !value.is_array() {
            return Err(ConvertError::new::<Self>(ctx, Some(&value)));
        }
        value
            .cast::<rhai::Array>()
            .into_iter()
            .map(|elem| from_rhai(&mut ctx, elem))
            .collect::<Result<Vec<T>, ConvertError>>()
            .in_structure("array")
    }
}

/// Implements [`FromRhai`] for a type.
#[macro_export]
macro_rules! impl_from_rhai {
    ($type:ty, $name:expr $(, $impl:expr)?) => {
        impl FromRhai for $type {
            fn expected_string() -> String {
                $name.to_owned()
            }

            $(
                fn try_from_rhai(ctx: impl RhaiCtx, value: Dynamic) -> Result<Self, ConvertError> {
                    let f: fn(_, Dynamic) -> Result<Self, ConvertError> = $impl;
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
impl_from_rhai!(bool, "bool");
impl_from_rhai!(char, "char");
impl_from_rhai!(f32, "number", |ctx, value| {
    f64::try_from_rhai(ctx, value).map(|x| x as f32)
});
impl_from_rhai!(f64, "number", |ctx, value| {
    None.or_else(|| value.as_float().ok())
        .or_else(|| value.as_int().ok().map(|i| i as f64))
        .ok_or_else(|| ConvertError::new::<Self>(ctx, Some(&value)))
});
impl_from_rhai!(i64, "integer", |ctx, value| {
    None.or_else(|| value.as_int().ok())
        .or_else(|| value.as_float().ok().and_then(hypermath::to_approx_integer))
        .ok_or_else(|| ConvertError::new::<Self>(ctx, Some(&value)))
});
impl_from_rhai!(usize, "index", |ctx, value| {
    (value.as_int().ok())
        .and_then(|i| i.try_into().ok())
        .ok_or_else(|| ConvertError::new::<Self>(ctx, Some(&value)))
});
impl_from_rhai!(u8, "small nonnegative integer", |ctx, value| {
    (value.as_int().ok())
        .and_then(|i| i.try_into().ok())
        .ok_or_else(|| ConvertError::new::<Self>(ctx, Some(&value)))
});
impl_from_rhai!(FnPtr, "function");

// Math types
impl_from_rhai!(Vector, "vector", |ctx, value| {
    Err(value)
        .or_else(|v| v.try_cast_result::<Vector>())
        .or_else(|v| try_cast_rhai_axis_to_vector(v))
        .map_err(|v| ConvertError::new::<Self>(ctx, Some(&v)))
});
impl_from_rhai!(Point, "point");
impl_from_rhai!(Blade, "vector, point, or PGA blade", |ctx, value| {
    Err(value)
        .or_else(|v| v.try_cast_result::<Blade>())
        .or_else(|v| v.try_cast_result().map(|p: Point| Blade::from_point(&p)))
        .or_else(|v| v.try_cast_result().map(|v: Vector| Blade::from_vector(v)))
        .or_else(|v| try_cast_rhai_axis_to_vector(v).map(Blade::from_vector))
        .map_err(|v| ConvertError::new::<Self>(ctx, Some(&v)))
});
impl_from_rhai!(Motor, "transform", |ctx, value| {
    Err(value)
        .or_else(|v| v.try_cast_result::<Motor>())
        // TODO: try cast to twist
        .map_err(|v| ConvertError::new::<Self>(ctx, Some(&v)))
});

fn try_cast_rhai_axis_to_vector(v: Dynamic) -> Result<Vector, Dynamic> {
    let ax = v.try_cast_result::<RhaiAxis>()?;
    ax.vector().map_err(|_| Dynamic::from(ax))
}

// Puzzle elements
impl_from_rhai!(RhaiAxis, "axis");
impl_from_rhai!(RhaiColor, "color");
impl_from_rhai!(RhaiTwist, "twist");

impl FromRhai for () {
    fn expected_string() -> String {
        "nothing".to_owned()
    }

    fn try_from_rhai(ctx: impl RhaiCtx, value: Dynamic) -> Result<Self, ConvertError> {
        if value.is_unit() {
            Ok(())
        } else {
            Err(ConvertError::new::<Self>(ctx, Some(&value)))
        }
    }

    fn try_from_none(_ctx: impl RhaiCtx) -> Result<Self, ConvertError> {
        Ok(())
    }
}

pub struct OptVecOrSingle<T>(pub Vec<T>);
impl<T: FromRhai> FromRhai for OptVecOrSingle<T> {
    fn expected_string() -> String {
        format!("{}, or array of them", T::expected_string())
    }

    fn try_from_rhai(ctx: impl RhaiCtx, value: Dynamic) -> Result<Self, ConvertError> {
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

    fn try_from_none(_ctx: impl RhaiCtx) -> Result<Self, ConvertError> {
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

use std::sync::Arc;

use crate::{Spanned, Type, Value, ValueData};

/// Generates `impl TypeOf for $ty { ... }`
macro_rules! impl_ty {
    ($ty:ty = $hps_ty:expr) => {
        impl TypeOf for $ty {
            fn hps_ty() -> Type {
                $hps_ty
            }
        }
    };
}

/// Returns the equivalent HPS type.
pub fn hps_ty<T: TypeOf>() -> Type {
    T::hps_ty()
}

/// Trait for Rust types that have an equivalent HPS type.
pub trait TypeOf {
    /// Returns the equivalent HPS type, which may be [`Type::Union`].
    fn hps_ty() -> Type;
}
impl<T: TypeOf + ?Sized> TypeOf for &'_ T {
    fn hps_ty() -> Type {
        T::hps_ty()
    }
}
impl<T: TypeOf> TypeOf for Spanned<T> {
    fn hps_ty() -> Type {
        T::hps_ty()
    }
}
impl<T: TypeOf> TypeOf for Option<T> {
    fn hps_ty() -> Type {
        Type::unify(Type::Null, T::hps_ty())
    }
}
impl<T: TypeOf + ?Sized> TypeOf for Arc<T> {
    fn hps_ty() -> Type {
        T::hps_ty()
    }
}

impl_ty!(Value = Type::Any);
impl_ty!(ValueData = Type::Any);

// Implementations for specific types are in [`super::from_value`].

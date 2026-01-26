//! Number of dimensions trait.

/// Number of dimensions.
pub trait Ndim {
    /// Returns the minimum number of dimensions required to represent the
    /// object.
    ///
    /// This method may return `0`.
    fn ndim(&self) -> u8;
}

macro_rules! impl_ndim_for_tuple {
    ($($generic_param:ident),+; $($index:tt),+) => {
        impl<$($generic_param: Ndim,)+> Ndim for ($($generic_param,)+) {
            fn ndim(&self) -> u8 {
                [$(self.$index.ndim(),)+].into_iter().max().unwrap_or(0)
            }
        }
    };
}
impl_for_tuples!(impl_ndim_for_tuple);

impl<T: Ndim> Ndim for Option<T> {
    fn ndim(&self) -> u8 {
        match self {
            Some(x) => x.ndim(),
            None => 0,
        }
    }
}

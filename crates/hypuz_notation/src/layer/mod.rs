//! Types for working with layer numbers.
//!
//! Generally an **axis** is a collection of non-overlapping layers, typically
//! with the same moves available. Layers on an axis are numbered increasing
//! starting from 1, the shallowest layer. Additionally, layers can be specified
//! using negative numbers, which start at -1, the deepest layer.
//!
//! For example, on any axis of a 3x3x3 Rubik's cube, there are three layers:
//!
//! - outer face (1 or -3)
//! - middle layer (2 or -2)
//! - opposite outer face (3 or -1)
//!
//! For most applications, negative layer numbers are irrelevant. In these
//! cases, use [`Layer`]. When you want to support negative layer numbers, use
//! [`SignedLayer`].
//!
//! ## Collections
//!
//! Two collection types are provided: [`LayerRange`] and [`LayerSet`].

mod mask;
mod range;
mod signed;
mod unsigned;

pub use mask::{LayerMask, LayerSetIter};
pub use range::LayerRange;
pub use signed::SignedLayer;
pub use unsigned::Layer;

macro_rules! impl_into_and_cmp {
    ($layer_type:ty, $int_type:ty) => {
        impl From<$layer_type> for $int_type {
            fn from(value: $layer_type) -> Self {
                value.0.get() as $int_type
            }
        }

        impl PartialEq<$int_type> for $layer_type {
            fn eq(&self, other: &$int_type) -> bool {
                <$int_type>::from(*self).eq(other)
            }
        }

        impl PartialOrd<$int_type> for $layer_type {
            fn partial_cmp(&self, other: &$int_type) -> Option<std::cmp::Ordering> {
                <$int_type>::from(*self).partial_cmp(other)
            }
        }
    };
}

impl_into_and_cmp!(Layer, i16);
impl_into_and_cmp!(Layer, i32);
impl_into_and_cmp!(Layer, i64);
impl_into_and_cmp!(Layer, isize);

impl_into_and_cmp!(Layer, u16);
impl_into_and_cmp!(Layer, u32);
impl_into_and_cmp!(Layer, u64);
impl_into_and_cmp!(Layer, usize);

impl_into_and_cmp!(SignedLayer, i16);
impl_into_and_cmp!(SignedLayer, i32);
impl_into_and_cmp!(SignedLayer, i64);
impl_into_and_cmp!(SignedLayer, isize);

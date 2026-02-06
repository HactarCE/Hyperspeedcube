//! Typed index collections.
//!
//! When handling many different kinds of indexes, it's useful to wrap them in
//! newtypes to avoid accidentally indexing a collection using the wrong index.
//! This module provides a trait for such newtype wrappers ("typed indexes"), a
//! helper macro for defining them, and several collections that use them:
//!
//! - value per index: [`TiVec`]
//! - dense set of indexes: [`TiMask`]
//! - sparse set of indexes: [`TiSet`]

pub use tinyset::Fits64;

mod index;
mod mask;
pub mod vec;

pub use index::{TypedIndex, TypedIndexIter};
pub use mask::TiMask;
pub use vec::TiVec;

pub use crate::error::{IndexOutOfRange, IndexOverflow};

/// Sparse set of indexes. See [`tinyset::Set64`] for details.
///
/// For a dense set, use [`TiMask`].
pub type TiSet<I> = tinyset::Set64<I>;

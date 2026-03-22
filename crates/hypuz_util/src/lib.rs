//! Utilities for Hyperspeedcube.

#[macro_use]
mod macros;
mod bitvec;
pub mod error;
#[cfg(feature = "serde")]
pub mod serde_impl;
pub mod ti;

pub use bitvec::{b16_string_to_bitvec, bitvec_to_b16_string};

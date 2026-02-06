//! Utilities for Hyperspeedcube.

#[macro_use]
mod macros;
pub mod error;
#[cfg(feature = "serde")]
pub mod serde_impl;
pub mod ti;

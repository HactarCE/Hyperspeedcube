//! Built-in functions and constants.

pub mod assertions;
pub mod bases;
pub mod catalog;
pub mod collections;
pub mod euclid;
pub mod math;
pub mod operators;
pub mod output;
pub mod strings;
pub mod types;
pub mod vec;

use crate::{Builtins, Result};

#[cfg(feature = "hyperpaths")]
const INCLUDE_DEBUG_FNS: bool = !hyperpaths::IS_OFFICIAL_BUILD;
#[cfg(not(feature = "hyperpaths"))]
const INCLUDE_DEBUG_FNS: bool = true;

/// Defines all base functionality that isn't related to the puzzle catalog.
pub fn define_base_in(builtins: &mut Builtins<'_>) -> Result<()> {
    assertions::define_in(builtins)?;
    bases::define_in(builtins)?;
    collections::define_in(builtins)?;
    euclid::define_in(builtins)?;
    math::define_in(builtins)?;
    operators::define_in(builtins)?;
    output::define_in(builtins)?;
    strings::define_in(builtins)?;
    types::define_in(builtins)?;
    vec::define_in(builtins)?;

    if INCLUDE_DEBUG_FNS {
        builtins.set_fns(hps_fns![("time", |_| -> u64 {
            std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_micros() as u64
        })])?;
    }

    Ok(())
}

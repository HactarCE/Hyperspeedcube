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

use crate::{Result, Scope};

#[cfg(feature = "hyperpaths")]
const INCLUDE_DEBUG_FNS: bool = !hyperpaths::IS_OFFICIAL_BUILD;
#[cfg(not(feature = "hyperpaths"))]
const INCLUDE_DEBUG_FNS: bool = true;

/// Defines all base functionality that isn't related to the puzzle catalog.
pub fn define_base_in(scope: &Scope) -> Result<()> {
    assertions::define_in(&scope)?;
    bases::define_in(&scope)?;
    collections::define_in(&scope)?;
    euclid::define_in(&scope)?;
    math::define_in(&scope)?;
    operators::define_in(&scope)?;
    output::define_in(&scope)?;
    strings::define_in(&scope)?;
    types::define_in(&scope)?;
    vec::define_in(&scope)?;

    if INCLUDE_DEBUG_FNS {
        scope.register_builtin_functions(hps_fns![("time", |_| -> u64 {
            std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_micros() as u64
        })])?;
    }

    Ok(())
}

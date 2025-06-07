//! Built-in functions and constants.

pub mod assertions;
pub mod bases;
pub mod collections;
pub mod euclid;
pub mod math;
pub mod operators;
pub mod output;
pub mod strings;
pub mod types;
pub mod vec;

use crate::{Result, Scope};

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
    Ok(())
}

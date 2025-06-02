use std::sync::Arc;

#[macro_use]
mod macros;
mod assertions;
mod bases;
mod collections;
mod constants;
mod euclid;
mod geometry;
mod math;
mod operators;
mod output;
mod strings;

use crate::{Result, Scope};

pub fn new_builtins_scope() -> Arc<Scope> {
    // IIFE to mimic try_block
    (|| -> Result<_> {
        let scope = Scope::new();
        assertions::define_in(&scope)?;
        bases::define_in(&scope)?;
        collections::define_in(&scope)?;
        constants::define_in(&scope)?;
        euclid::define_in(&scope)?;
        math::define_in(&scope)?;
        operators::define_in(&scope)?;
        output::define_in(&scope)?;
        geometry::define_in(&scope)?;
        strings::define_in(&scope)?;
        Ok(scope)
    })()
    .expect("error define built-ins")
}

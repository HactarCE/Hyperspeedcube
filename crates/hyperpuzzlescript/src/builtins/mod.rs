use std::sync::Arc;

#[macro_use]
mod macros;
mod assertions;
mod geometry;
mod operators;
mod output;

use crate::{Result, Scope};

pub fn new_builtins_scope() -> Arc<Scope> {
    // IIFE to mimic try_block
    (|| -> Result<_> {
        let scope = Scope::new();
        assertions::define_in(&scope)?;
        operators::define_in(&scope)?;
        output::define_in(&scope)?;
        geometry::define_in(&scope)?;
        Ok(scope)
    })()
    .expect("error define built-ins")
}

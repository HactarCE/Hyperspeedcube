//! Hyperpuzzle catalog functions.

use std::ops::{Deref, DerefMut};

use hypuz_core::{CatalogBuilder, Component};

mod color_systems;
mod generators;
mod puzzles;
pub mod tags;
mod twist_systems;

use crate::{Builtins, EvalRequestTx, Map, Result};

/// Adds the built-in functions.
pub fn define_in(
    builtins: &mut Builtins<'_>,
    catalog: &CatalogBuilder,
    eval_tx: &EvalRequestTx,
) -> Result<()> {
    color_systems::define_in(builtins, catalog, eval_tx)?;
    puzzles::define_in(builtins, catalog, eval_tx)?;
    twist_systems::define_in(builtins, catalog, eval_tx)?;
    Ok(())
}

/// Values exported by the Hyperpuzzlescript construction code.
#[derive(Debug, Default, Clone)]
pub struct HpsExports(pub Map);

impl<E> Component<E> for HpsExports {}

impl Deref for HpsExports {
    type Target = Map;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for HpsExports {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl HpsExports {
    /// Constructs an empty exports map.
    pub fn new() -> Self {
        Self(Map::new())
    }
}

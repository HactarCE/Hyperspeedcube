use std::fmt;
use std::sync::Arc;

use hyperpuzzle_core::catalog::GeneratorOutput;
use hyperpuzzle_core::{CatalogMetadata, Puzzle, TwistSystem};

use crate::{EvalCtx, EvalRequestTx, Map, Result};

/// Trait for engines (puzzle engines, twist system engines, etc.).
pub trait EngineCallback<T>: Send + Sync + fmt::Display {
    /// Returns the name under which to register the engine.
    ///
    /// The default implementation calls `self.to_string()`.
    fn name(&self) -> String {
        self.to_string()
    }

    /// Constructs a new catalog object from keyword arguments.
    #[expect(clippy::wrong_self_convention, clippy::new_ret_no_self)]
    fn new(
        &self,
        ctx: &mut EvalCtx<'_>,
        meta: CatalogMetadata,
        kwargs: Map,
        eval_tx: EvalRequestTx,
    ) -> Result<GeneratorOutput<T>>;
}

/// Callback for a puzzle engine.
pub type PuzzleEngineCallback = Arc<dyn EngineCallback<Puzzle>>;

/// Callback for a twist system engine.
pub type TwistSystemEngineCallback = Arc<dyn EngineCallback<TwistSystem>>;

use std::sync::Arc;

use hyperpuzzle_core::catalog::Generator;
use hyperpuzzle_core::{CatalogMetadata, Puzzle, TwistSystem};

use crate::{EvalCtx, EvalRequestTx, Map, Result};

/// Trait for engines (puzzle engines, twist system engines, etc.).
pub trait EngineCallback<T>: Send + Sync {
    /// Returns the name under which to register the engine.
    fn name(&self) -> String;

    /// Constructs a new catalog object from keyword arguments.
    #[expect(clippy::wrong_self_convention, clippy::new_ret_no_self)]
    fn new(
        &self,
        ctx: &mut EvalCtx<'_>,
        meta: CatalogMetadata,
        kwargs: Map,
        eval_tx: EvalRequestTx,
    ) -> Result<Generator<T>>;
}

/// Callback for a puzzle engine.
pub type PuzzleEngineCallback = Arc<dyn EngineCallback<Puzzle>>;

/// Callback for a twist system engine.
pub type TwistSystemEngineCallback = Arc<dyn EngineCallback<TwistSystem>>;

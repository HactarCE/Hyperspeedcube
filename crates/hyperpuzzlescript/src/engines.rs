use std::sync::Arc;

use hyperpuzzle_core::catalog::TwistSystemSpec;
use hyperpuzzle_core::{Catalog, PuzzleListMetadata, PuzzleSpec};

use crate::{EvalCtx, EvalRequestTx, Map, Result};

/// Trait for engines (puzzle engines, twist system engines, etc.).
pub trait EngineCallback<M, R>: Send + Sync {
    /// Returns the name under which to register the engine.
    fn name(&self) -> String;

    /// Constructs a new catalog object from metadata and excess named
    /// arguments.
    fn new(
        &self,
        ctx: &mut EvalCtx<'_>,
        meta: M,
        kwargs: Map,
        catalog: Catalog,
        eval_tx: EvalRequestTx,
    ) -> Result<R>;
}

/// Callback for a puzzle engine.
pub type PuzzleEngineCallback = Arc<dyn EngineCallback<PuzzleListMetadata, PuzzleSpec>>;

/// Callback for a twist system engine.
pub type TwistSystemEngineCallback = Arc<dyn EngineCallback<IdAndName, TwistSystemSpec>>;

/// ID and name for a catalog object.
pub struct IdAndName {
    /// ID
    pub id: String,
    /// Name
    pub name: String,
}

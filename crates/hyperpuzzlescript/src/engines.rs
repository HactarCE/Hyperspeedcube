use std::sync::Arc;

use hyperpuzzle_core::catalog::{BuildCtx, CatalogObject, Generator};
use hyperpuzzle_core::{CatalogMetadata, Puzzle, Redirectable, TwistSystem};

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
    ) -> Result<LazyCatalogConstructor<T>>;
}

/// Callback for a puzzle engine.
pub type PuzzleEngineCallback = Arc<dyn EngineCallback<Puzzle>>;

/// Callback for a twist system engine.
pub type TwistSystemEngineCallback = Arc<dyn EngineCallback<TwistSystem>>;

/// Type produced by an engine.
///
/// As much computation as possible should be deferred to happen inside `build`,
/// which will only be called if the object actually needs to be simulated. If
/// the object is displayed in a list, then only `meta` will be used.
pub struct LazyCatalogConstructor<T> {
    /// Metadata for object.
    pub meta: Arc<CatalogMetadata>,
    /// Callback to construct the object.
    pub build: Box<dyn Send + Sync + Fn(BuildCtx) -> eyre::Result<Arc<T>>>,
}

impl<T: CatalogObject> LazyCatalogConstructor<T> {
    /// Returns a generator with no parameters.
    pub fn into_generator(self) -> Generator<T> {
        Generator::new_lazy_constant(self.meta, move |build_ctx| {
            (self.build)(build_ctx).map(Redirectable::Direct)
        })
    }
}

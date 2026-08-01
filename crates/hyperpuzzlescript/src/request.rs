use std::sync::{Arc, mpsc};

use crate::{EvalCtx, FnValue, List, Map, Runtime, Scope, Span, Value};

/// Callback to execute with an HPS runtime.
pub type EvalRequest = Box<dyn Send + FnOnce(&mut Runtime)>;

/// Handle to access a thread with an HPS runtime.
///
/// This type is cheap to clone.
#[derive(Clone)]
pub struct EvalRequestTx(mpsc::Sender<EvalRequest>);

impl EvalRequestTx {
    /// Constructs a new channel for HPS eval requests.
    pub fn new() -> (Self, mpsc::Receiver<EvalRequest>) {
        let (tx, rx) = mpsc::channel();
        (Self(tx), rx)
    }

    /// Evaluates a callback on the HPS thread and returns the result of it.
    ///
    /// The callback is provided a [`Runtime`], from which an [`EvalCtx`] may be
    /// constructed if desired. [`Self::eval_blocking()`] is typically easier to
    /// use.
    ///
    /// This function is **not** re-entrant; if you call it or
    /// [`Self::eval_blocking()`] from within itself then it **will** deadlock.
    ///
    /// # Panics
    ///
    /// Panics if there are any issues communicating with the HPS thread.
    pub fn eval_blocking_raw<R, F>(&self, f: F) -> R
    where
        R: 'static + Send,
        F: 'static + Send + FnOnce(&mut Runtime) -> R,
    {
        let (result_tx, result_rx) = mpsc::sync_channel(0);
        self.0
            .send(Box::new(move |runtime| {
                if let Err(e) = result_tx.send(f(runtime)) {
                    eprintln!("error sending result from HPS thread: {e}");
                }
            }))
            .expect("error sending request to HPS thread");
        result_rx
            .recv()
            .expect("error receiving result from HPS thread")
    }

    /// Evaluates a callback on the HPS thread and returns the result of it.
    ///
    /// The callback is provided an [`EvalCtx`] constructed from
    /// [`crate::BUILTIN_SPAN`] whose exports are ignored.
    ///
    /// This function is **not** re-entrant; if you call it or
    /// [`Self::eval_blocking_raw()`] from within itself then it **will**
    /// deadlock.
    ///
    /// # Panics
    ///
    /// Panics if there are any issues communicating with the HPS thread.
    pub fn eval_blocking<R, F>(&self, scope: Arc<Scope>, f: F) -> R
    where
        R: 'static + Send,
        F: 'static + Send + FnOnce(&mut EvalCtx<'_>) -> R,
    {
        self.eval_blocking_raw(move |runtime| {
            f(&mut EvalCtx::new(
                &scope,
                runtime,
                crate::BUILTIN_SPAN,
                &mut None,
            ))
        })
    }
}

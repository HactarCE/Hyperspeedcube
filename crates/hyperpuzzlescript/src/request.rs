use std::sync::mpsc;

use crate::Runtime;

/// Handle to access a thread with an HPS runtime.
///
/// This type is cheap to clone.
#[derive(Clone)]
pub struct EvalRequestTx(mpsc::Sender<Box<dyn Send + Sync + FnOnce(&mut Runtime)>>);
impl EvalRequestTx {
    /// Constructs a new channel for HPS eval requests.
    pub fn new() -> (
        Self,
        mpsc::Receiver<Box<dyn Send + Sync + FnOnce(&mut Runtime)>>,
    ) {
        let (tx, rx) = mpsc::channel();
        (Self(tx), rx)
    }

    /// Evaluates a callback on the HPS thread and returns the result of it.
    ///
    /// This function is **not** re-entrant; if you call it from within itself
    /// then it **will** deadlock.
    ///
    /// # Panics
    ///
    /// Panics if there are any issues communicating with the HPS thread.
    pub fn eval_blocking<R, F>(&self, f: F) -> R
    where
        R: 'static + Send,
        F: 'static + Send + Sync + FnOnce(&mut Runtime) -> R,
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
            .expect("error received result from HPS thread")
    }
}

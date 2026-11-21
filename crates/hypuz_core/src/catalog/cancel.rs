use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering::Relaxed},
};

/// Flag used to signal that a task should be canceled.
///
/// Cloning creates another reference to the same token.
///
/// This is implemented simply as a wrapper around `Arc<AtomicBool>`.
#[derive(Debug, Default, Clone)]
pub struct CancelFlag(Arc<AtomicBool>);

impl CancelFlag {
    /// Returns a new uncanceled cancel flag.
    pub fn new() -> Self {
        Self::default()
    }

    /// Signals that the task should be canceled.
    pub fn cancel(&self) {
        self.0.store(true, Relaxed);
    }

    /// Returns whether the task should be canceled.
    pub fn is_canceled(&self) -> bool {
        self.0.load(Relaxed)
    }
}

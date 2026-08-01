use super::*;

/// Flag that wakes waiting threads when it is dropped.
///
/// # Example
///
/// ```rust
/// # use hyperpuzzle_core::catalog::NotifyWhenDropped;
/// let notify_when_dropped = NotifyWhenDropped::new();
///
/// let waiter = notify_when_dropped.waiter();
///
/// std::thread::spawn(move || {
///     waiter.wait();
///     println!("2");
/// });
///
/// println!("1");
/// drop(notify_when_dropped);
/// ```
#[derive(Debug, Default)]
pub struct NotifyWhenDropped(Arc<(Mutex<bool>, Condvar)>);

impl NotifyWhenDropped {
    /// Constructs a new notify-when-dropped flag.
    pub fn new() -> Self {
        Self::default()
    }
    /// Returns a handle to the flag that can be waited on.
    pub fn waiter(&self) -> Waiter {
        Waiter(Arc::clone(&self.0))
    }
}

impl Drop for NotifyWhenDropped {
    fn drop(&mut self) {
        let (mutex, condvar) = &*self.0;
        *mutex.lock() = true;
        condvar.notify_all();
    }
}

/// Handle to a [`NotifyWhenDropped`] flag.
#[derive(Debug, Clone)]
pub struct Waiter(Arc<(Mutex<bool>, Condvar)>);

impl Waiter {
    /// Waits until the flag is set.
    pub fn wait(self) {
        let (mutex, condvar) = &*self.0;
        condvar.wait_while(&mut mutex.lock(), |is_done| !*is_done);
    }

    /// Returns whether the flag is set.
    pub fn is_done(&self) -> bool {
        let (mutex, _condvar) = &*self.0;
        *mutex.lock()
    }
}

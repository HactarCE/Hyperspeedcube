use std::sync::Arc;

use parking_lot::{Condvar, Mutex};

use super::Redirectable;

/// Entry in the catalog.
///
/// If this is present, then some worker thread is working on building the
/// object.
#[derive(Debug, Default)]
pub enum CacheEntry<T> {
    /// There is a thread responsible for building the object, but it hasn't
    /// started yet. (It may be waiting on a mutex to unlock, for example.)
    #[default]
    NotStarted,
    /// The object is currently being build.
    Building {
        /// Progress on building the object.
        progress: Arc<Mutex<Progress>>,
        /// Flag to wake waiting threads when the object is built.
        notify: NotifyWhenDropped,
    },
    /// The object has been built.
    Ok(Redirectable<Arc<T>>),
    /// The object could not be built due to an error.
    Err(String),
}
impl<T> From<Result<Redirectable<Arc<T>>, String>> for CacheEntry<T> {
    fn from(value: Result<Redirectable<Arc<T>>, String>) -> Self {
        match value {
            Ok(ok) => Self::Ok(ok),
            Err(err) => Self::Err(err),
        }
    }
}

/// Flag that wakes waiting threads when it is dropped.
///
/// # Example
///
/// ```rust
/// let notify_when_dropped = NotifyWhenDropped::new();
///
/// let waiter = notify_when_dropped.waiter();
///
/// std::thread::spawn(move || {
///     waiter.wait();
///     println!("2");
/// });
///
/// println!("1")
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
}

/// Progress on building an object in the catalog.
#[derive(Debug, Default, Clone)]
pub struct Progress {
    /// Current task.
    pub task: BuildTask,
}

/// Current task while building a puzzle, color system, etc.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub enum BuildTask {
    /// Initializing data structures.
    #[default]
    Initializing,
    /// Generating the specification from a generator.
    GeneratingSpec,
    /// Building color system.
    BuildingColors,
    /// Building puzzle.
    BuildingPuzzle,
    /// Finalizing the object.
    Finalizing,
}

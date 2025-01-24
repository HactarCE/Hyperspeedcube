use std::fmt;
use std::sync::Arc;

use parking_lot::{Condvar, Mutex};

use super::Redirectable;

#[derive(Debug)]
pub enum CacheEntry<T> {
    NotStarted,
    Building {
        progress: Arc<Mutex<Progress>>,
        notify: NotifyWhenDropped,
    },
    Ok(Redirectable<Arc<T>>),
    Err(String), // TODO: what?
}
impl<T> Default for CacheEntry<T> {
    fn default() -> Self {
        CacheEntry::NotStarted
    }
}
impl<T> From<Result<Redirectable<Arc<T>>, String>> for CacheEntry<T> {
    fn from(value: Result<Redirectable<Arc<T>>, String>) -> Self {
        match value {
            Ok(ok) => Self::Ok(ok),
            Err(err) => Self::Err(err),
        }
    }
}

#[derive(Debug, Default)]
pub struct NotifyWhenDropped(Arc<(Mutex<bool>, Condvar)>);
impl NotifyWhenDropped {
    pub fn new() -> Self {
        Self::default()
    }
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

#[derive(Debug, Clone)]
pub struct Waiter(Arc<(Mutex<bool>, Condvar)>);
impl Waiter {
    pub fn wait(self) {
        let (mutex, condvar) = &*self.0;
        condvar.wait_while(&mut mutex.lock(), |is_done| !*is_done);
    }
}

#[derive(Debug, Default, Clone)]
pub struct Progress {
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
impl fmt::Display for BuildTask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BuildTask::Initializing => write!(f, "Initializing"),
            BuildTask::GeneratingSpec => write!(f, "Generating spec"),
            BuildTask::BuildingColors => write!(f, "Building color system"),
            BuildTask::BuildingPuzzle => write!(f, "Building"),
            BuildTask::Finalizing => write!(f, "Finalizing"),
        }
    }
}

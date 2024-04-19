use std::fmt;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;

use parking_lot::{Condvar, Mutex};

/// Handle to a task that will be completed asynchronously on another thread.
#[must_use]
pub struct TaskHandle<T>(Arc<TaskData<T>>);
impl<T> fmt::Debug for TaskHandle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TaskHandle")
            .field("completed", &self.0.completed)
            .field("cancel_requested", &self.0.cancel_requested)
            .finish()
    }
}

impl<T> Clone for TaskHandle<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T> TaskHandle<T> {
    /// Constructs a new task.
    pub(crate) fn new() -> Self {
        TaskHandle(Arc::new(TaskData::default()))
    }

    /// Marks the task as completed.
    pub(crate) fn complete(&self, result: T) {
        *self.0.result.lock() = Some(result);
        self.0.completed.store(true, Relaxed);
        self.0.condvar.notify_all();
    }
    /// Takes the result of the task, blocking until a result is ready.
    pub fn take_result_blocking(&self) -> T {
        let mut result = self.0.result.lock();
        loop {
            match result.take() {
                Some(result) => return result,
                None => self.0.condvar.wait(&mut result),
            }
        }
    }
    /// Takes the result of the task, or returns `None` if no result is ready
    /// yet or if the result has already been taken.
    pub fn take_result(&self) -> Option<T> {
        match self.0.completed.load(Relaxed) {
            true => self.0.result.lock().take(),
            false => None,
        }
    }
    /// Calls the callback when the task completes successfully, if that ever
    /// happens.
    pub fn on_complete(self, callback: impl 'static + Send + FnOnce(T))
    where
        T: 'static + Send,
    {
        std::thread::spawn(move || callback(self.take_result_blocking()));
    }
}

/// Task that will be completed asynchronously on another thread.
#[derive(Debug)]
pub struct TaskData<T> {
    /// Whether the task has been completed.
    completed: AtomicBool,
    /// Whether the task should be canceled.
    cancel_requested: AtomicBool,

    /// Result of the task, once it has been completed.
    result: Mutex<Option<T>>,
    condvar: Condvar,
}
impl<T> Default for TaskData<T> {
    fn default() -> Self {
        Self {
            completed: AtomicBool::new(false),
            cancel_requested: AtomicBool::new(false),

            result: Mutex::new(None),
            condvar: Condvar::new(),
        }
    }
}

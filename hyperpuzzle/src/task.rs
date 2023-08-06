use std::fmt;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;

use parking_lot::Mutex;

#[derive(Clone)]
pub struct TaskHandle<T>(Arc<TaskData<T>>);
impl<T> fmt::Debug for TaskHandle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TaskHandle")
            .field("completed", &self.0.completed)
            .field("cancel_requested", &self.0.cancel_requested)
            .finish()
    }
}

impl<T> TaskHandle<T> {
    pub fn cancel(&self) {
        self.0.cancel_requested.store(true, Relaxed);
    }
    pub fn set_on_update(&self, on_update: Option<Box<dyn Fn()>>) {
        *self.0.on_update.lock() = on_update
    }
    pub(crate) fn complete(&self, result: Option<T>) {
        *self.0.result.lock() = result;
        self.0.completed.store(true, Relaxed);
    }
    pub fn take_result(&self) -> Option<T> {
        match self.0.completed.load(Relaxed) {
            true => self.0.result.lock().take(),
            false => None,
        }
    }
}

#[derive(Default)]
pub struct TaskData<T> {
    on_update: Mutex<Option<Box<dyn Fn()>>>,
    completed: AtomicBool,
    cancel_requested: AtomicBool,

    result: Mutex<Option<T>>,
}

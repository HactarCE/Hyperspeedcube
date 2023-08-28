use std::fmt;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;

use parking_lot::{Condvar, Mutex, MutexGuard};

use crate::lua::LuaLogLine;

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
    pub(crate) fn new() -> Self {
        TaskHandle(Arc::new(TaskData::default()))
    }

    pub fn logs(&self) -> MutexGuard<'_, Vec<LuaLogLine>> {
        self.0.logs.lock()
    }
    pub fn cancel(&self) {
        self.0.cancel_requested.store(true, Relaxed);
    }
    pub(crate) fn complete(&self, result: T) {
        *self.0.result.lock() = Some(result);
        self.0.completed.store(true, Relaxed);
        self.0.condvar.notify_all();
    }
    pub fn take_result_blocking(&self) -> T {
        let mut result = self.0.result.lock();
        loop {
            match result.take() {
                Some(result) => return result,
                None => self.0.condvar.wait(&mut result),
            }
        }
    }
    pub fn take_result(&self) -> Option<T> {
        match self.0.completed.load(Relaxed) {
            true => self.0.result.lock().take(),
            false => None,
        }
    }
}

#[derive(Debug)]
pub struct TaskData<T> {
    completed: AtomicBool,
    cancel_requested: AtomicBool,

    logs: Mutex<Vec<LuaLogLine>>,

    result: Mutex<Option<T>>,
    condvar: Condvar,
}
impl<T> Default for TaskData<T> {
    fn default() -> Self {
        Self {
            completed: AtomicBool::new(false),
            cancel_requested: AtomicBool::new(false),

            logs: Mutex::new(vec![]),

            result: Mutex::new(None),
            condvar: Condvar::new(),
        }
    }
}

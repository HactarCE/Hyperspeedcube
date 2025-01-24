use std::sync::Arc;

pub use log::Level;
use parking_lot::{Mutex, MutexGuard};

#[derive(Debug, Default, Clone)]
pub struct Logger {
    lines: Arc<Mutex<Vec<LogLine>>>,
}
impl Logger {
    /// Constructs a new logger.
    pub fn new() -> Self {
        Self::default()
    }

    pub fn log(&self, line: LogLine) {
        self.lines.lock().push(line);
    }
    fn log_with_level(&self, level: Level, msg: String) {
        self.log(LogLine {
            level,
            file: None,
            msg,
            traceback: None,
        });
    }

    /// Logs an error line.
    pub fn error(&self, msg: impl ToString) {
        self.log_with_level(Level::Error, msg.to_string());
    }
    /// Logs an warn line.
    pub fn warn(&self, msg: impl ToString) {
        self.log_with_level(Level::Warn, msg.to_string());
    }
    /// Logs an info line.
    pub fn info(&self, msg: impl ToString) {
        self.log_with_level(Level::Info, msg.to_string());
    }
    /// Logs an debug line.
    pub fn debug(&self, msg: impl ToString) {
        self.log_with_level(Level::Debug, msg.to_string());
    }
    /// Logs an trace line.
    pub fn trace(&self, msg: impl ToString) {
        self.log_with_level(Level::Trace, msg.to_string());
    }

    // TODO: reconsider this API
    pub fn clear(&self) {
        self.lines.lock().clear();
    }
    // TODO: reconsider this API
    pub fn lines(&self) -> MutexGuard<'_, Vec<LogLine>> {
        self.lines.lock()
    }
}

/// Log line emitted by Lua code.
#[derive(Debug, Clone)]
pub struct LogLine {
    /// Log level.
    pub level: Level,
    /// Lua file that emitted the message.
    pub file: Option<String>,
    /// Log message.
    pub msg: String,
    /// Traceback.
    pub traceback: Option<String>,
}
impl LogLine {
    /// Returns whether the line matches a filter string entered by the user.
    pub fn matches_filter_string(&self, filter_string: &str) -> bool {
        filter_string.is_empty()
            || self
                .file
                .as_ref()
                .is_some_and(|file| file.contains(filter_string))
            || self.msg.contains(filter_string)
    }
}

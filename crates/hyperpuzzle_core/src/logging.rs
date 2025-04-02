use std::sync::Arc;

pub use log::Level;
use parking_lot::{Mutex, MutexGuard};

/// Logger for puzzle construction.
///
/// Only use this for information that you want the end user to see. If the
/// puzzle is working correctly, there should be no log entries. Prefer
/// conventional logging for other uses.
///
/// `hyperpuzzle_lua` has very specific logging needs that are not served well
/// by any established logging crates (namely the ability to store a file and
/// traceback unrelated to the line of Rust code that emitted the message) so we
/// use a custom logger.
#[derive(Debug, Default, Clone)]
pub struct Logger {
    lines: Arc<Mutex<Vec<LogLine>>>,
}
impl Logger {
    /// Constructs a new logger.
    pub fn new() -> Self {
        Self::default()
    }

    /// Logs a line.
    pub fn log(&self, line: LogLine) {
        self.lines.lock().push(line);
    }
    fn log_with_level(&self, level: Level, msg: String) {
        self.log(LogLine { level, msg });
    }

    /// Logs a line with [`Level::Error`] and no file or traceback.
    pub fn error(&self, msg: impl ToString) {
        self.log_with_level(Level::Error, msg.to_string());
    }
    /// Logs a line with [`Level::Warn`] and no file or traceback.
    pub fn warn(&self, msg: impl ToString) {
        self.log_with_level(Level::Warn, msg.to_string());
    }
    /// Logs a line with [`Level::Info`] and no file or traceback.
    pub fn info(&self, msg: impl ToString) {
        self.log_with_level(Level::Info, msg.to_string());
    }
    /// Logs a line with [`Level::Debug`] and no file or traceback.
    pub fn debug(&self, msg: impl ToString) {
        self.log_with_level(Level::Debug, msg.to_string());
    }
    /// Logs a line with [`Level::Trace`] and no file or traceback.
    pub fn trace(&self, msg: impl ToString) {
        self.log_with_level(Level::Trace, msg.to_string());
    }

    /// Clear all log lines.
    pub fn clear(&self) {
        self.lines.lock().clear();
    }
    /// Returns all the log lines so far.
    pub fn lines(&self) -> MutexGuard<'_, Vec<LogLine>> {
        self.lines.lock()
    }
}

/// Log line emitted by Lua code.
#[derive(Debug, Clone)]
pub struct LogLine {
    /// Log level.
    pub level: Level,
    /// Log message.
    pub msg: String,
}
impl LogLine {
    /// Returns whether the line matches a filter string entered by the user.
    pub fn matches_filter_string(&self, filter_string: &str) -> bool {
        filter_string.is_empty() || self.msg.contains(filter_string)
    }
}

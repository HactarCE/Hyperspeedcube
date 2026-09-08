use std::sync::Arc;

pub use log::Level;
use parking_lot::{Mutex, MutexGuard};

/// Logger for puzzle construction.
///
/// Only use this for information that you want the end user to see. If the
/// puzzle is working correctly, there should be no log entries. Prefer
/// conventional logging for other uses.
///
/// `hyperpuzzlescript` has specific logging needs that are not served well by
/// any established logging crates so we use a custom logger.
#[derive(Debug, Default, Clone)]
pub struct Logger {
    lines: Arc<Mutex<Vec<LogLine>>>,
}
impl PartialEq for Logger {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.lines, &other.lines)
    }
}
impl Eq for Logger {}
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
        self.log(LogLine {
            level,
            filename: None,
            msg,
            full: None,
        });
    }

    /// Logs a line with [`Level::Error`] and no filename.
    pub fn error(&self, msg: impl ToString) {
        self.log_with_level(Level::Error, msg.to_string());
    }
    /// Logs a line with [`Level::Warn`] and no filename.
    pub fn warn(&self, msg: impl ToString) {
        self.log_with_level(Level::Warn, msg.to_string());
    }
    /// Logs a line with [`Level::Info`] and no filename.
    pub fn info(&self, msg: impl ToString) {
        self.log_with_level(Level::Info, msg.to_string());
    }
    /// Logs a line with [`Level::Debug`] and no filename.
    pub fn debug(&self, msg: impl ToString) {
        self.log_with_level(Level::Debug, msg.to_string());
    }
    /// Logs a line with [`Level::Trace`] and no filename.
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

/// Log line emitted by a puzzle backend.
#[derive(Debug, Clone)]
pub struct LogLine {
    /// Log level.
    pub level: Level,
    /// Filename, if any.
    pub filename: Option<String>,
    /// Brief log message.
    pub msg: String,
    /// Full error message, if any.
    ///
    /// This may use ANSI escape codes for setting text foreground color.
    pub full: Option<String>,
}
impl LogLine {
    /// Returns whether the line matches a filter string entered by the user.
    pub fn matches_filter_string(&self, filter_string: &str) -> bool {
        filter_string.is_empty()
            || self
                .filename
                .as_ref()
                .is_some_and(|f| f.contains(filter_string))
            || self.msg.contains(filter_string)
    }
}

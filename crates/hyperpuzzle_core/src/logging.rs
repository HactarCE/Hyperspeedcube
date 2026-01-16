use std::sync::{Arc, Mutex, MutexGuard};

pub use log::Level;

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
impl Logger {
    /// Constructs a new logger.
    pub fn new() -> Self {
        Self::default()
    }

    /// Logs a line.
    pub fn log(&self, line: LogLine) {
        self.lines.lock().unwrap().push(line);
    }
    fn log_with_level(&self, level: Level, msg: String) {
        self.log(LogLine {
            level,
            msg,
            full: None,
        });
    }

    /// Logs a line with [`Level::Error`].
    pub fn error(&self, msg: impl ToString) {
        self.log_with_level(Level::Error, msg.to_string());
    }
    /// Logs a line with [`Level::Warn`].
    pub fn warn(&self, msg: impl ToString) {
        self.log_with_level(Level::Warn, msg.to_string());
    }
    /// Logs a line with [`Level::Info`].
    pub fn info(&self, msg: impl ToString) {
        self.log_with_level(Level::Info, msg.to_string());
    }
    /// Logs a line with [`Level::Debug`].
    pub fn debug(&self, msg: impl ToString) {
        self.log_with_level(Level::Debug, msg.to_string());
    }
    /// Logs a line with [`Level::Trace`].
    pub fn trace(&self, msg: impl ToString) {
        self.log_with_level(Level::Trace, msg.to_string());
    }

    /// Clear all log lines.
    pub fn clear(&self) {
        self.lines.lock().unwrap().clear();
    }
    /// Returns all the log lines so far.
    pub fn lines(&self) -> MutexGuard<'_, Vec<LogLine>> {
        self.lines.lock().unwrap()
    }
}

/// Log line emitted by a puzzle backend.
#[derive(Debug, Clone)]
pub struct LogLine {
    /// Log level.
    pub level: Level,
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
        filter_string.is_empty() || self.msg.contains(filter_string)
    }
}

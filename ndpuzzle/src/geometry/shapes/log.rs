use itertools::Itertools;
use parking_lot::Mutex;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tinyset::{Fits64, Set64};

const INDENT: &str = "  ";

#[derive(Default, Clone)]
pub(super) struct ShapeConstructionLog(Arc<Mutex<LogInner>>);
impl fmt::Debug for ShapeConstructionLog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ShapeConstructionLog")
            .finish_non_exhaustive()
    }
}
impl fmt::Display for ShapeConstructionLog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.lock().log_lines)
    }
}
impl ShapeConstructionLog {
    pub fn event(&self, event_type: &'static str, msg: impl fmt::Display) -> EventGuard {
        let mut this = self.0.lock();
        let ev = EventGuard {
            log: self.clone(),
            event_id: this.next_event_id,
            event_type,
            indent_level: this.next_indent_level,
            initial_log_len: AtomicUsize::new(0),
        };
        this.next_event_id += 1;
        this.next_indent_level += 1;
        drop(this);
        ev.log_line_with_char(Some('*'), msg);
        ev
    }
}

#[derive(Debug, Default, Clone)]
struct LogInner {
    log_lines: String,
    next_event_id: usize,
    next_indent_level: usize,
}
impl LogInner {
    fn log_line(
        &mut self,
        indent_level: usize,
        prefix_char: Option<char>,
        event_id: usize,
        event_type: &str,
        msg: impl fmt::Display,
    ) {
        self.indent(indent_level);
        if let Some(ch) = prefix_char {
            self.log_lines += &format!("[{ch} {event_id}:{event_type}] {msg}\n");
        } else {
            self.log_lines += &format!("  [{event_id}:{event_type}] {msg}\n");
        }
    }
    fn indent(&mut self, indent_level: usize) {
        for _ in 0..indent_level {
            self.log_lines.push_str(INDENT);
        }
    }
}

pub(super) struct EventGuard {
    log: ShapeConstructionLog,
    event_id: usize,
    event_type: &'static str,
    indent_level: usize,
    /// Length of the log after the initial message of this event. If the log
    /// moves past this point, then there were event details or sub-events, so
    /// we should add another message to indicate the end of this event.
    initial_log_len: AtomicUsize,
}
impl Drop for EventGuard {
    fn drop(&mut self) {
        if self.log.0.lock().log_lines.len() > *self.initial_log_len.get_mut() {
            self.log_line_with_char(Some('#'), "end");
        }
        let mut log = self.log.0.lock();
        log.next_indent_level -= 1;
    }
}
impl EventGuard {
    fn log_line_with_char(&self, ch: Option<char>, msg: impl fmt::Display) {
        let mut log = self.log.0.lock();
        log.log_line(self.indent_level, ch, self.event_id, self.event_type, msg);
        if ch == Some('*') {
            self.initial_log_len
                .store(log.log_lines.len(), Ordering::Relaxed);
        }
    }

    pub fn log(&self, msg: impl fmt::Display) {
        self.log_line_with_char(None, msg);
    }
    pub fn log_value(&self, var_name: &str, value: impl fmt::Display) {
        self.log(format!("? {var_name} = {value}"));
    }
    pub fn log_option(&self, var_name: &str, value: Option<impl fmt::Display>) {
        match value {
            Some(value) => self.log_value(var_name, value),
            None => self.log_value(var_name, "None"),
        }
    }
    pub fn log_set64<T: fmt::Display + Fits64>(&self, var_name: &str, value: &Set64<T>) {
        self.log_value(var_name, format!("[{}]", value.iter().join(", ")));
    }
}

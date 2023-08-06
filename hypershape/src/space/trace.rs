use std::cell::RefCell;

thread_local! {
    static TRACER: Option<RefCell<Tracer>> = None;
}

struct Tracer {
    /// Log file indent level.
    indent: AtomicUsize,
    /// Span stack.
    span_stack: Mutex<Vec<Span>>,
    /// Recorded log lines.
    log_lines: Mutex<Vec<LogLine>>,
}

struct Span {
    id: usize,
    name: String,
}

struct LogLine {
    level: tracing::Level,
    id: usize,
    msg: String,
    begin_end: Option<LogLineBeginEnd>,
}

enum Level {
    Print,
    Warn,
    Error,
}

enum LogLineBeginEnd {
    Begin,
    End,
}

use std::sync::mpsc;

use itertools::Itertools;
use mlua::prelude::*;

use super::{lua_current_filename, lua_stack_trace};

/// Lua logging facade.
#[derive(Debug, Clone)]
pub struct LuaLogger {
    tx: mpsc::Sender<LuaLogLine>,
}
impl LuaLogger {
    /// Constructs a new logger.
    pub fn new() -> (Self, mpsc::Receiver<LuaLogLine>) {
        let (tx, rx) = mpsc::channel();
        (Self { tx }, rx)
    }

    fn send(&self, line: LuaLogLine) {
        self.tx.send(line).expect("error in Lua logging");
    }

    /// Logs a message.
    fn log(&self, lua: &Lua, level: LuaLogLevel, msg: String) {
        self.send(LuaLogLine {
            level,
            file: lua_current_filename(lua),
            msg,
            traceback: Some(lua_stack_trace(lua)),
        });
    }

    /// Logs an info line.
    pub fn info(&self, lua: &Lua, msg: impl ToString) {
        self.log(lua, LuaLogLevel::Info, msg.to_string());
    }
    /// Logs a warning.
    pub fn warn(&self, lua: &Lua, msg: impl ToString) {
        self.log(lua, LuaLogLevel::Warn, msg.to_string());
    }
    /// Logs an error.
    pub fn error(&self, file: Option<String>, msg: impl ToString) {
        self.send(LuaLogLine {
            level: LuaLogLevel::Error,
            file,
            msg: msg.to_string(),
            traceback: None,
        });
    }

    /// Returns a Lua function that calls `string.format()` on its arguments and
    /// then logs the result as an info line.
    pub(super) fn lua_info_fn(&self, lua: &Lua) -> LuaResult<LuaFunction> {
        let this = self.clone();
        lua.create_function(move |lua, args: LuaMultiValue| {
            let args: Vec<String> = args.iter().map(|arg| arg.to_string()).try_collect()?;
            this.info(lua, args.into_iter().join("\t"));
            Ok(())
        })
    }
}

/// Log line emitted by Lua code.
#[derive(Debug, Clone)]
pub struct LuaLogLine {
    /// Log level.
    pub level: LuaLogLevel,
    /// Lua file that emitted the message.
    pub file: Option<String>,
    /// Log message.
    pub msg: String,
    /// Traceback.
    pub traceback: Option<String>,
}
impl LuaLogLine {
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

/// Log level of a Lua log line.
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash)]
pub enum LuaLogLevel {
    /// Normal print.
    #[default]
    Info,
    /// Warning.
    Warn,
    /// Fatal error.
    Error,
}

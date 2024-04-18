use std::sync::mpsc;

use itertools::Itertools;
use mlua::prelude::*;

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

    /// Logs a message.
    pub(super) fn log(&self, level: LuaLogLevel, file: Option<String>, msg: String) {
        self.tx
            .send(LuaLogLine { msg, file, level })
            .expect("error in Lua logging");
    }

    /// Logs an info line.
    pub fn info(&self, file: Option<String>, msg: impl ToString) {
        self.log(LuaLogLevel::Info, file, msg.to_string());
    }
    /// Logs a warning.
    pub fn warn(&self, file: Option<String>, msg: impl ToString) {
        self.log(LuaLogLevel::Warn, file, msg.to_string());
    }
    /// Logs an error.
    pub fn error(&self, file: Option<String>, msg: impl ToString) {
        self.log(LuaLogLevel::Error, file, msg.to_string());
    }

    /// Returns a Lua function that calls `string.format()` on its arguments and
    /// then logs the result as an info line.
    pub(super) fn lua_info_fn<'lua>(&self, lua: &'lua Lua) -> LuaResult<LuaFunction<'lua>> {
        let this = self.clone();
        lua.create_function(move |lua, args: LuaMultiValue<'_>| {
            let file = crate::lua::current_filename(lua);
            let args: Vec<String> = args.iter().map(|arg| arg.to_string()).try_collect()?;
            this.info(file, args.into_iter().join("\t"));
            Ok(())
        })
    }
}

/// Log line emitted by Lua code.
#[derive(Debug, Clone)]
pub struct LuaLogLine {
    /// Log message.
    pub msg: String,
    /// Lua file that emitted the message.
    pub file: Option<String>,
    /// Log level.
    pub level: LuaLogLevel,
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

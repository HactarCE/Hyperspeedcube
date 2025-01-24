use std::path::PathBuf;

use mlua::prelude::*;

/// Lua file.
#[derive(Debug)]
pub(super) struct LuaFile {
    /// Name of the file. This is arbitrary but must be unique.
    pub name: String,
    /// The path to the file. If specified, this may be used to reload the file
    /// if it changes.
    pub path: Option<PathBuf>,
    /// Contents of the file. This should be valid Lua code if this is a real
    /// file.
    pub contents: Option<String>,
    /// State of the file: unloaded, loading, or loaded (with either an error or
    /// a table of exports).
    pub load_state: LuaFileLoadState,
}
impl PartialEq for LuaFile {
    fn eq(&self, other: &Self) -> bool {
        // Ignore load state and dependents when comparing files.
        self.name == other.name && self.path == other.path && self.contents == other.contents
    }
}
impl LuaFile {
    pub fn start_loading(&mut self) {
        self.load_state = LuaFileLoadState::Loading;
    }

    /// Finish loading the file.
    ///
    /// Returns an error if the file is not currently being loaded.
    pub fn finish_loading(&mut self, result: LuaResult<LuaTable>) {
        self.load_state = LuaFileLoadState::Loaded(result.and_then(|exports_table| {
            if matches!(self.load_state, LuaFileLoadState::Loading) {
                Ok(exports_table)
            } else {
                Err(LuaError::external(format!("bad file state: {self:?}")))
            }
        }));
    }
}

#[derive(Debug, Default, Clone)]
pub(super) enum LuaFileLoadState {
    #[default]
    Unloaded,
    Loading,
    Loaded(LuaResult<LuaTable>),
}

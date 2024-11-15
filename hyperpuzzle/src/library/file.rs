use std::path::PathBuf;

use mlua::prelude::*;

/// File stored in a [`super::Library`].
#[derive(Debug)]
pub struct LibraryFile {
    /// Name of the file. This may be chosen arbitrarily by the calling code,
    /// and may include some or all of the path.
    pub name: String,
    /// The path to the file. If specified, this may be used to reload the file
    /// if it changes.
    pub path: Option<PathBuf>,
    /// Contents of the file. This should be valid Lua code if this is a real
    /// file.
    pub contents: Option<String>,

    /// State of the file: unloaded, loading, or loaded (with either an error or
    /// a table of exports).
    pub(crate) load_state: LibraryFileLoadState,
}
impl PartialEq for LibraryFile {
    fn eq(&self, other: &Self) -> bool {
        // Ignore load state and dependents when comparing files.
        self.name == other.name && self.path == other.path && self.contents == other.contents
    }
}
impl LibraryFile {
    pub(crate) fn start_loading(&mut self) {
        self.load_state = LibraryFileLoadState::Loading;
    }

    /// Finish loading the file.
    ///
    /// Returns an error if the file is not currently being loaded.
    pub(crate) fn finish_loading(&mut self, result: LuaResult<LuaTable>) {
        self.load_state = LibraryFileLoadState::Loaded(result.and_then(|exports_table| {
            if matches!(self.load_state, LibraryFileLoadState::Loading) {
                Ok(exports_table)
            } else {
                Err(LuaError::external(format!("bad file state: {self:?}")))
            }
        }));
    }
}

#[derive(Debug, Default, Clone)]
pub(crate) enum LibraryFileLoadState {
    #[default]
    Unloaded,
    Loading,
    Loaded(LuaResult<LuaTable>),
}

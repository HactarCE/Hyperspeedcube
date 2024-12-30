use std::path::PathBuf;

/// Command sent to the thread with the Lua interpreter.
pub(crate) enum LibraryCommand {
    /// Clears all Lua files.
    Reset,
    /// Read Lua files from a directory and add them to the library.
    ReadDirectory { directory: PathBuf },
    /// Add a Lua file to the library.
    AddFile {
        filename: String,
        path: Option<PathBuf>,
        contents: String,
    },
    /// Execute all Lua files that haven't been executed yet.
    LoadFiles,
    /// Build a puzzle that has already been loaded from a Lua file.
    BuildPuzzle { id: String },
    /// Wait for the puzzle library to complete the tasks assigned to it.
    Wait(std::sync::mpsc::SyncSender<()>), // TODO: is this useful?
}

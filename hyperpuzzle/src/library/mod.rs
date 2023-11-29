mod command;
mod frontend;

use command::LibraryCommand;
pub use frontend::Library;

#[derive(Debug, Clone)]
pub struct PuzzleData {
    name: String,
}

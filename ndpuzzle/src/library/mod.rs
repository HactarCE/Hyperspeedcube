use std::collections::HashMap;

mod lua;

use anyhow::Result;
use rlua::{Lua, Table};

/// Puzzle loader + list of loaded puzzles.
pub struct PuzzleLibrary {
    lua: Lua,
    files: HashMap<String, String>,
    puzzles: Vec<PuzzleTypeInfo>,
}
impl PuzzleLibrary {
    pub fn new() -> Self {
        Self {
            lua: lua::new_lua(),
            files: HashMap::new(),
            puzzles: vec![],
        }
    }

    pub fn load_file(&mut self, filename: String, contents: String) {
        let result: Result<()> = self.lua.context(|ctx| {
            let sandbox_env: Table = ctx
                .globals()
                .get("SANDBOX_ENV")
                .expect("missing sandbox environment");
            ctx.load(&contents).set_environment(sandbox_env)?.exec()?;
            // ctx.globals()
            //     .get("NEW_PUZZLES")
            //     .expect("msising puzzles list")
            Ok(())
        });
        if let Err(e) = result {
            todo!("TODO handle Lua errors")
        }
        self.files.insert(filename, contents);
    }
}

/// Metadata about a puzzle, including its name, description, relation to other
/// puzzles, and data needed to construct it.
pub struct PuzzleTypeInfo {
    name: String,
    tags: Vec<String>,
}

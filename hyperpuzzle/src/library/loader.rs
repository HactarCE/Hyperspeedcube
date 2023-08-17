use std::sync::Arc;

use anyhow::Result;
use parking_lot::RwLock;
use rlua::prelude::*;

use super::{FileData, LibraryCommand, ObjectStore};

pub(super) struct ObjectLoader {
    lua: Lua,
    store: Arc<RwLock<ObjectStore>>,
}
impl ObjectLoader {
    pub(super) fn new(store: Arc<RwLock<ObjectStore>>) -> Self {
        Self {
            lua: crate::lua::new_lua(),
            store,
        }
    }

    pub(super) fn store(&self) -> &Arc<RwLock<ObjectStore>> {
        &self.store
    }

    pub(super) fn do_command(&mut self, command: LibraryCommand) {
        match command {
            LibraryCommand::LoadFile {
                filename,
                contents,
                progress,
            } => {
                let result = self.load_file(filename, contents);
                progress.complete(result);
            }

            LibraryCommand::ConstructPuzzle { name, progress } => {
                self.store.read().construct_puzzle(progress, &name);
            }
        }
    }

    fn load_file(&mut self, filename: String, contents: String) -> Result<()> {
        let objects = crate::lua::load_sandboxed(&self.lua, &filename, &contents)?;
        let mut puzzle_list = self.store.write();
        puzzle_list.update_file(FileData {
            name: filename,
            contents,
            objects,
            dependencies: vec![], // TODO: dependencies
        });
        Ok(())
    }
}

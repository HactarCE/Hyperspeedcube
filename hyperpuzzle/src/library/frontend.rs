use std::sync::{mpsc, Arc};

use anyhow::{anyhow, Result};
use parking_lot::{RwLock, RwLockReadGuard};

use super::{LibraryCommand, ObjectLoader, ObjectStore};
use crate::TaskHandle;

/// Handle to a library of puzzles and puzzle-related objects. This type is
/// cheap to `Clone` (just an `mpsc::Sender` and `Arc`).
///
/// All loading happens asynchronously on other threads.
#[derive(Debug, Clone)]
pub struct Library {
    tx: mpsc::Sender<LibraryCommand>,
    store: Arc<RwLock<ObjectStore>>,
}

impl Default for Library {
    fn default() -> Self {
        Self::new()
    }
}

impl Library {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let store = ObjectStore::new();
        let mut loader = ObjectLoader::new(Arc::clone(&store));

        std::thread::spawn(move || {
            for command in rx {
                loader.do_command(command);
            }
        });

        Library { tx, store }
    }

    pub fn load_file(&self, filename: String, contents: String) -> TaskHandle<Result<()>> {
        let task = TaskHandle::new();
        let command = LibraryCommand::LoadFile {
            filename,
            contents,
            progress: task.clone(),
        };
        self.send_command(command, task)
    }

    fn send_command<T>(
        &self,
        command: LibraryCommand,
        task: TaskHandle<Result<T>>,
    ) -> TaskHandle<Result<T>> {
        if let Err(e) = self.tx.send(command) {
            task.complete(Err(anyhow!(e)));
        }
        task
    }

    pub fn try_get_puzzles(&self) -> Option<RwLockReadGuard<ObjectStore>> {
        self.store.try_read()
    }
}

use std::collections::HashMap;
use std::sync::{Arc, Weak};

use anyhow::{Context, Result};
use parking_lot::RwLock;

use crate::{Object, ObjectData, Puzzle, TaskHandle};

/// Storage for loaded objects. These objects haven't been constructed, but we
/// know some metadata like the number of dimensions of each object and which
/// file it came from.
///
/// This also contains a cache of constructed puzzles.
#[derive(Debug, Default, Clone)]
pub struct ObjectStore {
    this: Weak<RwLock<Self>>,
    files: HashMap<String, FileData>,
    file_of_each_object: HashMap<String, String>,
    constructed_puzzles_cache: HashMap<String, Weak<Puzzle>>,
}
impl ObjectStore {
    pub fn new() -> Arc<RwLock<Self>> {
        Arc::new_cyclic(|this| {
            RwLock::new(ObjectStore {
                this: Weak::clone(this),
                files: HashMap::new(),
                file_of_each_object: HashMap::new(),
                constructed_puzzles_cache: HashMap::new(),
            })
        })
    }

    pub fn this(&self) -> Arc<RwLock<Self>> {
        self.this
            .upgrade()
            .expect("failed to upgrade object store reference")
    }

    pub(super) fn update_file(&mut self, file_data: FileData) {
        // Unload old file.
        self.unload_file(&file_data.name);

        // Load new file.
        for obj in &file_data.objects {
            self.file_of_each_object
                .insert(obj.id.clone(), file_data.name.clone());
        }
        self.files.insert(file_data.name.clone(), file_data);
    }
    fn unload_file(&mut self, filename: &str) {
        if let Some(old) = self.files.remove(filename) {
            for obj in old.objects {
                self.file_of_each_object.remove(&obj.id);
            }
        }
    }

    /// Returns the name of the file that defined an object, or `None` if the
    /// object isn't loaded.
    pub fn get_file_containing_definition(&self, obj_id: &str) -> Option<&FileData> {
        let filename = self.file_of_each_object.get(obj_id)?;
        self.files.get(filename)
    }

    pub(super) fn construct_puzzle(&self, task: TaskHandle<Result<Arc<Puzzle>>>, name: &str) {
        let store = self.this();
        let name = name.to_string();

        std::thread::spawn(move || {
            // IIFE to mimic try_block
            let result = (|| {
                let store_reader = store.read();
                let file = store_reader
                    .get_file_containing_definition(&format!("puzzle[{name:?}]")) // TODO: this relies on Lua and Rust string escaping being the same
                    .with_context(|| format!("no puzzle named {name:?}"))?
                    .clone(); // TODO: instead of cloning, construct dependency graph
                drop(store_reader);

                let lua = crate::lua::new_lua();
                // IIFE to mimic try_block
                let result = (|| {
                    crate::lua::load_sandboxed(&lua, &file.name, &file.contents)
                        .with_context(|| format!("error loading file {:?}", file.name))?;
                    let puzzle = lua.context(|lua| crate::lua::build_puzzle(lua, &name))?;
                    store
                        .write()
                        .constructed_puzzles_cache
                        .insert(name, Arc::downgrade(&puzzle));
                    Ok(puzzle)
                })();

                *task.logs() = lua.context(crate::lua::drain_logs);

                result
            })();
            task.complete(result);
        });
    }

    pub fn puzzles(&self) -> Vec<String> {
        self.files
            .values()
            .flat_map(|file| {
                file.objects
                    .iter()
                    .filter(|obj| matches!(obj.data, ObjectData::Puzzle { .. }))
                    .map(|obj| obj.name.clone())
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct FileData {
    pub name: String,
    pub contents: String,
    pub objects: Vec<Object>,
    pub dependencies: Vec<String>,
}

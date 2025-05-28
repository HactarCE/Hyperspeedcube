use std::fmt;
use std::path::Path;
use std::sync::Arc;

use arcstr::{ArcStr, Substr};
use indexmap::IndexMap;
use lazy_static::lazy_static;

#[cfg(feature = "hyperpaths")]
use crate::LANGUAGE_NAME;
use crate::{FILE_EXTENSION, FileId, INDEX_FILE_NAME, Result, Value, ast};

lazy_static! {
    static ref INTERNAL_SOURCE: ariadne::Source<ArcStr> =
        ariadne::Source::from(ArcStr::from("<internal>"));
}

#[derive(Debug)]
pub struct Module {
    pub file_path: Substr,
    pub submodules: Vec<FileId>,
    pub contents: ArcStr,
    pub source: ariadne::Source<ArcStr>,
    pub ast: Option<Arc<ast::Node>>,
    pub result: Option<Result<Value, ()>>,
}
impl Module {
    fn new(file_path: Substr, source: ArcStr) -> Self {
        Self {
            file_path,
            submodules: vec![],
            contents: source.clone(),
            source: ariadne::Source::from(source),
            ast: None,
            result: None,
        }
    }
}

/// Source files.
///
/// The path of a file is its location on disk, relative to the root folder.
/// Example: `puzzles/ft_cubic.hps`
///
/// The path of a module is the same as its file path, without the trailing
/// `/index.hps` or `.hps` extension. Modules are stored in a key-value store
/// where the key is their path.
///
/// Examples:
///
/// | File path               | Module path        |
/// | ----------------------- | ------------------ |
/// | `test.hps`              | `test`             |
/// | `puzzles/ft_cubic.hps`  | `puzzles/ft_cubic` |
/// | `piece_types/index.hps` | `piece_types`      |
///
/// If `some_module/index.hps` and `some_module.hps` both exist, then only one
/// of them is used and an error is logged using the global logging
/// infrastructure.
#[derive(Debug, Default)]
pub struct Modules(IndexMap<Substr, Module>);

impl Modules {
    /// Constructs a new file store with built-in files and user files (if
    /// feature `hyperpaths` is enabled).
    pub fn with_default_files() -> Self {
        let mut ret = Self::default();

        // Load built-in files.
        ret.add_builtin_files();

        // Load user files.
        #[cfg(feature = "hyperpaths")]
        match hyperpaths::hps_dir() {
            Ok(hps_dir) => {
                log::info!(
                    "reading {LANGUAGE_NAME} files from path {}",
                    hps_dir.to_string_lossy(),
                );
                ret.add_from_directory(hps_dir);
            }
            Err(e) => log::error!("error locating {LANGUAGE_NAME} directory: {e}"),
        }

        ret
    }

    /// Adds built-in files to the file store.
    pub fn add_builtin_files(&mut self) {
        let mut stack = vec![crate::HPS_BUILTIN_DIR.clone()];
        while let Some(dir) = stack.pop() {
            for entry in dir.entries() {
                match entry {
                    include_dir::DirEntry::Dir(subdir) => stack.push(subdir.clone()),
                    include_dir::DirEntry::File(file) => {
                        let path = file.path();
                        if path.extension().is_some_and(|ext| ext == FILE_EXTENSION) {
                            match file.contents_utf8() {
                                Some(contents) => self.add_file(path, contents),
                                None => log::error!("error loading built-in file {path:?}"),
                            }
                        }
                    }
                }
            }
        }
    }

    /// Adds files recursively from a directory on disk.
    pub fn add_from_directory(&mut self, directory: &std::path::Path) {
        for entry in walkdir::WalkDir::new(directory).follow_links(true) {
            match entry {
                Ok(entry) => {
                    let path = entry.path();
                    if path.extension().is_some_and(|ext| ext == FILE_EXTENSION) {
                        let relative_path = path.strip_prefix(directory).unwrap_or(path);
                        match std::fs::read_to_string(path) {
                            Ok(contents) => self.add_file(relative_path, contents),
                            Err(e) => log::error!("error loading file {relative_path:?}: {e}"),
                        }
                    }
                }
                Err(e) => log::warn!("error reading filesystem entry: {e:?}"),
            }
        }
    }

    /// Adds a file to the file store and returns the `FileId`.
    ///
    /// `path` must be relative to the script directory.
    pub fn add_file(&mut self, path: &Path, contents: impl Into<ArcStr>) {
        let file_path = Substr::from(path.to_string_lossy());

        let mut module_path = path.with_extension("");
        if module_path.ends_with(INDEX_FILE_NAME) {
            module_path.pop();
        }

        let new_file_id;
        let mut module = Module::new(file_path, contents.into());
        match self.0.entry(Substr::from(module_path.to_string_lossy())) {
            indexmap::map::Entry::Occupied(e) if !e.get().contents.is_empty() => {
                new_file_id = e.index() as FileId;
                log::warn!(
                    "files {:?} and {:?} have the same module path of {:?}",
                    e.get().file_path,
                    module.file_path,
                    module_path.clone(),
                );
            }
            indexmap::map::Entry::Occupied(mut e) => {
                new_file_id = e.index() as FileId;
                module.submodules = std::mem::take(&mut e.get_mut().submodules);
                e.insert(module);
            }
            indexmap::map::Entry::Vacant(e) => {
                new_file_id = e.index() as FileId;
                e.insert(module);
            }
        };

        let mut file_id_of_child = new_file_id;
        for parent in module_path.ancestors().skip(1) {
            let dir_str = Substr::from(parent.to_string_lossy());
            match self.0.entry(dir_str.clone()) {
                indexmap::map::Entry::Occupied(mut e) => {
                    e.get_mut().submodules.push(file_id_of_child);
                    break;
                }
                indexmap::map::Entry::Vacant(e) => {
                    let new_id = e.index() as FileId;
                    let m = e.insert(Module::new(dir_str, ArcStr::new()));
                    m.submodules.push(file_id_of_child);
                    file_id_of_child = new_id;
                }
            }
        }
    }

    /// Returns whether `path` exists in the module tree.
    pub fn has_module(&self, path: &str) -> bool {
        self.0.contains_key(path)
    }

    /// Returns the ID of the file containing the given module.
    pub fn id_from_module_name(&self, path: &str) -> Option<FileId> {
        Some(self.0.get_index_of(path)? as FileId)
    }

    /// Returns the path of a file.
    pub fn get_path(&self, id: FileId) -> Option<&str> {
        Some(&self.0.get_index(id as usize)?.1.file_path)
    }
    /// Returns the contents of a file.
    pub fn get_contents(&self, id: FileId) -> Option<&ArcStr> {
        Some(&self.0.get_index(id as usize)?.1.contents)
    }
    /// Returns the module name for a file (last component of the module path).
    pub fn module_name(&self, id: FileId) -> Option<Substr> {
        let module_path = self.0.get_index(id as usize)?.0;
        match module_path.rsplit_once('/') {
            Some((_parent_path, child_name)) => Some(module_path.substr_from(child_name)),
            None => Some(module_path.substr(..)),
        }
    }

    pub(crate) fn get(&mut self, id: FileId) -> Option<&Module> {
        Some(self.0.get_index(id as usize)?.1)
    }
    pub(crate) fn get_mut(&mut self, id: FileId) -> Option<&mut Module> {
        Some(self.0.get_index_mut(id as usize)?.1)
    }

    /// Returns whether the file store is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    /// Returns the number of files in the store.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub(crate) fn ariadne_source(
        &self,
        id: FileId,
    ) -> Result<&ariadne::Source<ArcStr>, impl fmt::Debug> {
        if id == FileId::MAX {
            return Ok(&INTERNAL_SOURCE);
        }
        match self.0.get_index(id as usize) {
            Some((_path, file)) => Ok(&file.source),
            None => Err(Box::new(format!("no file with ID {id}"))),
        }
    }
    pub(crate) fn ariadne_display(&self, id: FileId) -> Option<String> {
        if id == FileId::MAX {
            return Some("<builtin>".to_owned());
        }
        self.get_path(id).map(str::to_owned)
    }
}

impl ariadne::Cache<FileId> for &Modules {
    type Storage = ArcStr;

    fn fetch(&mut self, id: &FileId) -> Result<&ariadne::Source<ArcStr>, impl fmt::Debug> {
        self.ariadne_source(*id)
    }

    fn display<'a>(&self, id: &'a FileId) -> Option<impl fmt::Display + 'a> {
        self.ariadne_display(*id)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn test_file_store() {
        let mut mods = Modules::default();
        mods.add_file(&PathBuf::from("dir1.hps"), "this is the index");
        mods.add_file(&PathBuf::from("dir1/dir2/hello.hps"), "hello, world!");
        mods.add_file(&PathBuf::from("dir1/dir3/index.hps"), "dir3 index");

        let root = mods.id_from_module_name("").unwrap();
        let dir1 = mods.id_from_module_name("dir1").unwrap();
        let dir2 = mods.id_from_module_name("dir1/dir2").unwrap();
        let hello = mods.id_from_module_name("dir1/dir2/hello").unwrap();
        let dir3 = mods.id_from_module_name("dir1/dir3").unwrap();
        assert_eq!(mods.0.len(), 5);

        assert_eq!(mods.module_name(root).unwrap(), "");
        let f = mods.get_mut(root).unwrap();
        assert_eq!(f.submodules, vec![dir1]);
        assert_eq!(f.file_path, "");
        assert_eq!(f.contents, "");

        assert_eq!(mods.module_name(dir1).unwrap(), "dir1");
        let f = mods.get_mut(dir1).unwrap();
        assert_eq!(f.submodules, vec![dir2, dir3]);
        assert_eq!(f.file_path, "dir1.hps");
        assert_eq!(f.contents, "this is the index");

        assert_eq!(mods.module_name(dir2).unwrap(), "dir2");
        let f = mods.get_mut(dir2).unwrap();
        assert_eq!(f.submodules, vec![hello]);
        assert_eq!(f.file_path, "dir1/dir2");
        assert_eq!(f.contents, "");

        assert_eq!(mods.module_name(hello).unwrap(), "hello");
        let f = mods.get_mut(hello).unwrap();
        assert_eq!(f.submodules, vec![]);
        assert_eq!(f.file_path, "dir1/dir2/hello.hps");
        assert_eq!(f.contents, "hello, world!");

        assert_eq!(mods.module_name(dir3).unwrap(), "dir3");
        let f = mods.get_mut(dir3).unwrap();
        assert_eq!(f.submodules, vec![]);
        assert_eq!(f.file_path, "dir1/dir3/index.hps");
        assert_eq!(f.contents, "dir3 index");
    }
}

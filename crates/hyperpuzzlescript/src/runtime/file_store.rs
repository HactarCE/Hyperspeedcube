use std::{fmt, ops::Index, path::Path};

use indexmap::IndexMap;
use itertools::Itertools;

use crate::{FileId, Span};

/// Hyperpuzzlescript source files.
#[derive(Debug, Default)]
pub struct FileStore(IndexMap<String, ariadne::Source>);

impl FileStore {
    /// Constructs a new file store with built-in files and user files (if
    /// feature `hyperpaths` is enabled).
    pub(crate) fn with_default_files() -> Self {
        let mut ret = Self(IndexMap::new());

        // Load built-in files.
        ret.add_builtin_files();

        // Load user files.
        #[cfg(feature = "hyperpaths")]
        match hyperpaths::hps_dir() {
            Ok(hps_dir) => {
                log::info!(
                    "reading Hyperpuzzlescript files from path {}",
                    hps_dir.to_string_lossy(),
                );
                files.load_from_directory(hps_dir);
            }
            Err(e) => log::error!("error locating Hyperpuzzlescript directory: {e}"),
        }

        ret
    }

    /// Adds built-in files to the file store.
    fn add_builtin_files(&mut self) {
        let mut stack = vec![crate::HPS_BUILTIN_DIR.clone()];
        while let Some(dir) = stack.pop() {
            for entry in dir.entries() {
                match entry {
                    include_dir::DirEntry::Dir(subdir) => stack.push(subdir.clone()),
                    include_dir::DirEntry::File(file) => {
                        let path = file.path();
                        if path.extension().is_some_and(|ext| ext == "hps") {
                            match file.contents_utf8().map(str::to_owned) {
                                Some(contents) => self.add_file(&path, contents),
                                None => log::error!("error loading built-in file {path:?}"),
                            }
                        }
                    }
                }
            }
        }
    }

    /// Adds files recursively from a directory on disk.
    #[cfg(feature = "hyperpaths")]
    fn add_from_directory(&mut self, directory: &std::path::Path) {
        for entry in walkdir::WalkDir::new(directory).follow_links(true) {
            match entry {
                Ok(entry) => {
                    let path = entry.path();
                    if path.extension().is_some_and(|ext| ext == "rhai") {
                        let relative_path = path.strip_prefix(directory).unwrap_or(path);
                        match std::fs::read_to_string(path) {
                            Ok(contents) => self.add_file(relative_path.to_owned(), contents),
                            Err(e) => log::error!("error loading file {relative_path:?}: {e}"),
                        }
                    }
                }
                Err(e) => log::warn!("error reading filesystem entry: {e:?}"),
            }
        }
    }

    /// Adds a file to the file store.
    pub(crate) fn add_file(&mut self, path: &Path, contents: String) {
        let path_string = path
            .with_extension("")
            .components()
            .map(|path_component| path_component.as_os_str().to_string_lossy())
            .join("/")
            .chars()
            .filter(|&c| c != '"' && c != '\\') // dubious chars
            .collect();
        self.0.insert(path_string, ariadne::Source::from(contents));
    }

    /// Returns the name of a file.
    pub fn file_name(&self, id: FileId) -> Option<&str> {
        Some(self.0.get_index(id as usize)?.0)
    }
    /// Returns the contents of a file.
    pub fn file_contents(&self, id: FileId) -> Option<&str> {
        Some(self.0.get_index(id as usize)?.1.text())
    }

    /// Returns the number of files in the store.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub(crate) fn ariadne_source(&self, id: FileId) -> Result<&ariadne::Source, impl fmt::Debug> {
        match self.0.get_index(id as usize) {
            Some((_name, source)) => Ok(source),
            None => Err(Box::new(format!("no file with ID {id}"))),
        }
    }
    pub(crate) fn ariadne_display(&self, id: FileId) -> Option<String> {
        self.file_name(id).map(|s| s.to_owned())
    }
}

impl Index<Span> for FileStore {
    type Output = str;

    fn index(&self, span: Span) -> &Self::Output {
        match self.file_contents(span.context) {
            Some(contents) => &contents[span.start as usize..span.end as usize],
            None => "",
        }
    }
}

impl ariadne::Cache<FileId> for &FileStore {
    type Storage = String;

    fn fetch(&mut self, id: &FileId) -> Result<&ariadne::Source, impl fmt::Debug> {
        self.ariadne_source(*id)
    }

    fn display<'a>(&self, id: &'a FileId) -> Option<impl fmt::Display + 'a> {
        self.ariadne_display(*id)
    }
}

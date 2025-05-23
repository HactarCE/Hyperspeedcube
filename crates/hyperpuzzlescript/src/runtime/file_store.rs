use std::fmt;
use std::ops::Index;
use std::path::Path;
use std::sync::Arc;

use arcstr::{ArcStr, Substr};
use indexmap::IndexMap;
use itertools::Itertools;
use lazy_static::lazy_static;

#[cfg(feature = "hyperpaths")]
use crate::LANGUAGE_NAME;
use crate::{FILE_EXTENSION, FileId, Result, Span, Value, ast};

lazy_static! {
    static ref INTERNAL_SOURCE: ariadne::Source<ArcStr> =
        ariadne::Source::from(ArcStr::from("<internal>"));
}

#[derive(Debug)]
pub struct File {
    pub contents: ArcStr,
    pub source: ariadne::Source<ArcStr>,
    pub ast: Option<Arc<ast::Node>>,
    pub result: Option<Result<Value, ()>>,
}
impl File {
    fn new(source: ArcStr) -> Self {
        Self {
            contents: source.clone(),
            source: ariadne::Source::from(source),
            ast: None,
            result: None,
        }
    }
}

/// Source files.
///
/// The "name" of a file is typically a relative path using `/` as separator and
/// excluding the file extension.
#[derive(Debug, Default)]
pub struct FileStore(IndexMap<String, File>);

impl FileStore {
    /// Constructs a new file store with built-in files and user files (if
    /// feature `hyperpaths` is enabled).
    pub fn with_default_files() -> Self {
        let mut ret = Self(IndexMap::new());

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

    /// Adds a file to the file store.
    pub fn add_file(&mut self, path: &Path, contents: impl Into<ArcStr>) {
        let path_string = path
            .with_extension("")
            .components()
            .map(|path_component| path_component.as_os_str().to_string_lossy())
            .join("/")
            .chars()
            .filter(|&c| c != '"' && c != '\\') // dubious chars
            .collect();
        self.0.insert(path_string, File::new(contents.into()));
    }

    /// Returns the ID of the file with the given name.
    pub fn id_from_name(&self, name: &str) -> Option<FileId> {
        Some(self.0.get_index_of(name)? as FileId)
    }

    /// Returns the name of a file.
    pub fn file_name(&self, id: FileId) -> Option<&str> {
        Some(self.0.get_index(id as usize)?.0)
    }
    /// Returns the contents of a file.
    pub fn file_contents(&self, id: FileId) -> Option<&ArcStr> {
        Some(&self.0.get_index(id as usize)?.1.contents)
    }

    pub(crate) fn get_mut(&mut self, id: FileId) -> Option<&mut File> {
        Some(self.0.get_index_mut(id as usize)?.1)
    }

    /// Returns a [`Substr`] from `span`.
    pub fn substr(&self, span: Span) -> Substr {
        match self.file_contents(span.context) {
            Some(contents) => contents.substr(span.start as usize..span.end as usize),
            None => Substr::new(),
        }
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
            Some((_name, file)) => Ok(&file.source),
            None => Err(Box::new(format!("no file with ID {id}"))),
        }
    }
    pub(crate) fn ariadne_display(&self, id: FileId) -> Option<String> {
        if id == FileId::MAX {
            return Some("<builtin>".to_owned());
        }
        self.file_name(id).map(|s| format!("{s}.{FILE_EXTENSION}"))
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
    type Storage = ArcStr;

    fn fetch(&mut self, id: &FileId) -> Result<&ariadne::Source<ArcStr>, impl fmt::Debug> {
        self.ariadne_source(*id)
    }

    fn display<'a>(&self, id: &'a FileId) -> Option<impl fmt::Display + 'a> {
        self.ariadne_display(*id)
    }
}
